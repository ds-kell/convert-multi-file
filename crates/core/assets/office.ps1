<#
.SYNOPSIS
    Converts one document with Microsoft Office through COM automation.

.DESCRIPTION
    Called by ConvertApp (crates/core/src/converters/office/msoffice.rs);
    not meant to be run by hand. The document is opened read-only, saved in
    the requested format, then Office is closed again - also when something
    fails. Errors are written to stderr and the exit code is 1.

.PARAMETER App
    word, excel or powerpoint.
.PARAMETER Source
    Absolute path of the document to open.
.PARAMETER Target
    Absolute path of the file to write (overwritten).
.PARAMETER Format
    "pdf", "txt" or the numeric Office file-format code for SaveAs.
.PARAMETER PidFile
    Receives the id of the Office process started by this script, so the
    caller can kill it if the script hangs (e.g. on an unexpected dialog).
#>
param(
    [Parameter(Mandatory)][ValidateSet('word', 'excel', 'powerpoint')][string]$App,
    [Parameter(Mandatory)][string]$Source,
    [Parameter(Mandatory)][string]$Target,
    [Parameter(Mandatory)][string]$Format,
    [Parameter(Mandatory)][string]$PidFile
)

$ErrorActionPreference = 'Stop'

# A password no real document uses: protected files then fail with an error
# instead of showing a password prompt nobody can answer.
$NoPassword = 'convertapp-not-the-password'

$processName = @{ word = 'WINWORD'; excel = 'EXCEL'; powerpoint = 'POWERPNT' }[$App]
$before = @(Get-Process $processName -ErrorAction SilentlyContinue | ForEach-Object Id)

$office = $null
$doc = $null
# Word and Excel always start a private instance. PowerPoint is single
# instance: if the user already has it open, we must not quit it.
$ownInstance = $true
$failure = $null

try {
    switch ($App) {
        'word' {
            $office = New-Object -ComObject Word.Application
            $office.Visible = $false
            $office.DisplayAlerts = 0            # wdAlertsNone
        }
        'excel' {
            $office = New-Object -ComObject Excel.Application
            $office.Visible = $false
            $office.DisplayAlerts = $false
            $office.AskToUpdateLinks = $false
        }
        'powerpoint' {
            $ownInstance = ($before.Count -eq 0)
            $office = New-Object -ComObject PowerPoint.Application
        }
    }

    $started = @(Get-Process $processName -ErrorAction SilentlyContinue |
        Where-Object { $before -notcontains $_.Id } | ForEach-Object Id)
    if ($started.Count -gt 0) { Set-Content -LiteralPath $PidFile -Value $started[0] }

    switch ($App) {
        'word' {
            # Open(FileName, ConfirmConversions, ReadOnly, AddToRecentFiles, PasswordDocument)
            $doc = $office.Documents.Open($Source, $false, $true, $false, $NoPassword)
            if ($Format -eq 'pdf') {
                $doc.ExportAsFixedFormat($Target, 17)                 # wdExportFormatPDF
            }
            elseif ($Format -eq 'txt') {
                # SaveAs2(FileName, FileFormat=wdFormatText, ..., Encoding=UTF-8)
                $doc.SaveAs2($Target, 2, $false, '', $false, '', $false, $false, $false, $false, $false, 65001)
            }
            else {
                $doc.SaveAs2($Target, [int]$Format)
            }
        }
        'excel' {
            if ([IO.Path]::GetExtension($Source) -ieq '.csv') {
                # Excel opens .csv files in the system code page with the
                # locale's list separator and ignores OpenText options for
                # that extension. Open a .txt copy instead, explicitly as
                # UTF-8 (65001), delimited, double-quoted, comma-separated.
                $csvCopy = Join-Path ([IO.Path]::GetDirectoryName($PidFile)) 'source.txt'
                Copy-Item -LiteralPath $Source -Destination $csvCopy
                # OpenText(Filename, Origin, StartRow, DataType=xlDelimited,
                #   TextQualifier=xlDoubleQuote, ConsecutiveDelimiter, Tab, Semicolon, Comma)
                $office.Workbooks.OpenText($csvCopy, 65001, 1, 1, 1, $false, $false, $false, $true)
                $doc = $office.ActiveWorkbook
            }
            else {
                # Open(Filename, UpdateLinks=0, ReadOnly, Format, Password)
                $doc = $office.Workbooks.Open($Source, 0, $true, [Type]::Missing, $NoPassword)
            }
            if ($Format -eq 'pdf') {
                $doc.ExportAsFixedFormat(0, $Target)                  # xlTypePDF
            }
            else {
                $doc.SaveAs($Target, [int]$Format)
            }
        }
        'powerpoint' {
            # Open(FileName, ReadOnly=msoTrue, Untitled=msoFalse, WithWindow=msoFalse)
            $doc = $office.Presentations.Open($Source, -1, 0, 0)
            $code = if ($Format -eq 'pdf') { 32 } else { [int]$Format }   # 32 = ppSaveAsPDF
            $doc.SaveAs($Target, $code)
        }
    }
}
catch {
    $failure = $_.Exception.Message
}
finally {
    if ($null -ne $doc) {
        try {
            switch ($App) {
                'word' { $doc.Close(0) }                            # wdDoNotSaveChanges
                'excel' { $doc.Close($false) }
                'powerpoint' { $doc.Close() }
            }
        }
        catch { }
        [void][Runtime.InteropServices.Marshal]::ReleaseComObject($doc)
    }
    if ($null -ne $office) {
        if ($ownInstance) { try { $office.Quit() } catch { } }
        [void][Runtime.InteropServices.Marshal]::ReleaseComObject($office)
    }
    [GC]::Collect()
    [GC]::WaitForPendingFinalizers()
}

if ($null -ne $failure) {
    [Console]::Error.WriteLine($failure)
    exit 1
}
exit 0
