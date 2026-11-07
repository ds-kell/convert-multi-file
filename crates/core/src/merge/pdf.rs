//! Concatenating PDF documents (and images turned into PDF pages) with
//! [`lopdf`].
//!
//! Each source document is renumbered so its object ids do not collide with
//! the others, its pages are re-parented under one new page tree, and a new
//! catalog points at that tree. Page attributes that a page normally
//! inherits from its parent node (size, crop box, rotation, resources) are
//! copied onto the page first, since the old parent nodes are dropped.
//! Bookmarks, forms and other document-level features of the sources are not
//! carried over; the page content is.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use lopdf::{Dictionary, Document, Object, ObjectId};

use super::{is_image, load_image};
use crate::converters::image::pdf::PdfBuilder;
use crate::{ConvertError, ConvertOptions, Format, Result};

/// Page attributes that may be inherited from ancestor page-tree nodes.
const INHERITABLE: [&[u8]; 4] = [b"MediaBox", b"CropBox", b"Resources", b"Rotate"];

/// Builds one PDF from `inputs` (with their detected `formats`): images
/// become one page each, PDFs contribute all their pages. Calls `step(n)`
/// after the n-th input.
pub(super) fn merge_to_pdf(
    inputs: &[PathBuf],
    formats: &[Format],
    options: &ConvertOptions,
    step: &dyn Fn(usize),
) -> Result<Vec<u8>> {
    // Only images: build the PDF directly, no need to parse anything.
    if formats.iter().all(|&f| is_image(f)) {
        let mut builder = PdfBuilder::new(&options.image);
        for (index, (input, &format)) in inputs.iter().zip(formats).enumerate() {
            let image = load_image(input, format, options.image.resize)?;
            builder
                .add_image(&image)
                .map_err(|e| ConvertError::encode(input, e))?;
            step(index + 1);
        }
        return Ok(builder.finish());
    }

    let mut documents = Vec::with_capacity(inputs.len());
    for (index, (input, &format)) in inputs.iter().zip(formats).enumerate() {
        let document = if format == Format::Pdf {
            load_pdf(input)?
        } else {
            // An image becomes a one-page PDF first.
            let image = load_image(input, format, options.image.resize)?;
            let mut builder = PdfBuilder::new(&options.image);
            builder
                .add_image(&image)
                .map_err(|e| ConvertError::encode(input, e))?;
            Document::load_mem(&builder.finish()).map_err(|e| ConvertError::encode(input, e))?
        };
        documents.push(document);
        step(index + 1);
    }

    let first = &inputs[0];
    let mut merged = concatenate(documents).map_err(|e| ConvertError::encode(first, e))?;
    let mut bytes = Vec::new();
    merged
        .save_to(&mut bytes)
        .map_err(|e| ConvertError::encode(first, e))?;
    Ok(bytes)
}

/// Opens a PDF, refusing password-protected ones with a clear message.
fn load_pdf(path: &Path) -> Result<Document> {
    let document = Document::load(path).map_err(|e| ConvertError::decode(path, e))?;
    if document.is_encrypted() {
        return Err(ConvertError::decode(
            path,
            "the PDF is password-protected; remove the protection first",
        ));
    }
    Ok(document)
}

/// Joins `documents` page by page, keeping their order.
fn concatenate(documents: Vec<Document>) -> std::result::Result<Document, lopdf::Error> {
    let mut merged = Document::with_version("1.7");
    let mut page_ids: Vec<ObjectId> = Vec::new();
    let mut next_id = 1;

    for mut document in documents {
        // Copy inherited attributes onto each page before its parent goes.
        for page_id in document.get_pages().into_values() {
            let page = with_inherited_attributes(&document, page_id)?;
            document.objects.insert(page_id, Object::Dictionary(page));
        }
        document.renumber_objects_with(next_id);
        next_id = document.max_id + 1;

        // `get_pages` is keyed by page number, so this keeps reading order.
        page_ids.extend(document.get_pages().into_values());

        for (id, object) in document.objects {
            match object.type_name().unwrap_or(b"") {
                // Replaced by the new catalog and page tree below.
                b"Catalog" | b"Pages" => {}
                // Bookmarks point into the old structure: not carried over.
                b"Outlines" | b"Outline" => {}
                _ => {
                    merged.objects.insert(id, object);
                }
            }
        }
    }

    let pages_id = (next_id, 0);
    let catalog_id = (next_id + 1, 0);
    for &page_id in &page_ids {
        if let Ok(page) = merged.get_dictionary_mut(page_id) {
            page.set("Parent", pages_id);
        }
    }

    let mut pages = Dictionary::new();
    pages.set("Type", "Pages");
    pages.set("Count", page_ids.len() as i64);
    pages.set(
        "Kids",
        page_ids
            .iter()
            .copied()
            .map(Object::Reference)
            .collect::<Vec<_>>(),
    );
    merged.objects.insert(pages_id, Object::Dictionary(pages));

    let mut catalog = Dictionary::new();
    catalog.set("Type", "Catalog");
    catalog.set("Pages", pages_id);
    merged
        .objects
        .insert(catalog_id, Object::Dictionary(catalog));

    merged.trailer.set("Root", catalog_id);
    merged.max_id = next_id + 1;
    merged.renumber_objects();
    merged.compress();
    Ok(merged)
}

/// The page dictionary with every inheritable attribute resolved, searching
/// up the page tree like a PDF reader would.
fn with_inherited_attributes(
    document: &Document,
    page_id: ObjectId,
) -> std::result::Result<Dictionary, lopdf::Error> {
    let mut page = document.get_dictionary(page_id)?.clone();
    for key in INHERITABLE {
        if let Some(value) = inherited(document, page_id, key)? {
            page.set(key.to_vec(), value);
        }
    }
    // A page without any MediaBox is invalid; US Letter is the PDF default.
    if !page.has(b"MediaBox") {
        page.set("MediaBox", vec![0.into(), 0.into(), 612.into(), 792.into()]);
    }
    Ok(page)
}

/// First value of `key` on the page or one of its ancestors.
fn inherited(
    document: &Document,
    page_id: ObjectId,
    key: &[u8],
) -> std::result::Result<Option<Object>, lopdf::Error> {
    let mut current = page_id;
    let mut visited = HashSet::new();
    loop {
        if !visited.insert(current) {
            return Ok(None); // malformed tree with a cycle: stop searching
        }
        let node = document.get_dictionary(current)?;
        if let Ok(value) = node.get(key)
            && !matches!(document.dereference(value), Ok((_, Object::Null)) | Err(_))
        {
            return Ok(Some(value.clone()));
        }
        match node.get(b"Parent").and_then(Object::as_reference) {
            Ok(parent) => current = parent,
            Err(_) => return Ok(None),
        }
    }
}
