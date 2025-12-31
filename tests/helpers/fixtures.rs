use std::io::Write;

/// Create a minimal valid PNG image
pub fn create_test_png(width: u32, height: u32) -> Vec<u8> {
    // Create a minimal PNG using the image crate if available,
    // or return a hardcoded minimal PNG
    
    // For testing, we'll create a simple 1x1 PNG programmatically
    // This is a minimal valid PNG structure
    let mut png = Vec::new();
    
    // PNG signature
    png.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
    
    // IHDR chunk
    let ihdr_data_len = 13u32;
    png.extend_from_slice(&ihdr_data_len.to_be_bytes());
    png.extend_from_slice(b"IHDR");
    
    // Width and height (simplified - using provided dimensions)
    let w = width.min(65535) as u32; // PNG max width
    let h = height.min(65535) as u32; // PNG max height
    png.extend_from_slice(&w.to_be_bytes());
    png.extend_from_slice(&h.to_be_bytes());
    
    // Bit depth, color type, compression, filter, interlace
    png.extend_from_slice(&[8, 2, 0, 0, 0]); // 8-bit, RGB, no compression, no filter, no interlace
    
    // CRC for IHDR (simplified - in real PNG this would be calculated)
    // For tests, we'll use a placeholder
    let ihdr_crc = calculate_crc(&png[png.len() - 17..]);
    png.extend_from_slice(&ihdr_crc.to_be_bytes());
    
    // IDAT chunk with minimal image data
    let idat_data = vec![0; (w * h * 3) as usize]; // RGB data
    let idat_len = idat_data.len() as u32;
    png.extend_from_slice(&idat_len.to_be_bytes());
    png.extend_from_slice(b"IDAT");
    png.extend_from_slice(&idat_data);
    
    // CRC for IDAT
    let idat_crc_start = png.len() - idat_data.len() - 4;
    let idat_crc = calculate_crc(&png[idat_crc_start..png.len() - 4]);
    png.extend_from_slice(&idat_crc.to_be_bytes());
    
    // IEND chunk
    png.extend_from_slice(&[0, 0, 0, 0]); // Length 0
    png.extend_from_slice(b"IEND");
    let iend_crc = 0xAE426082u32; // Standard IEND CRC
    png.extend_from_slice(&iend_crc.to_be_bytes());
    
    png
}

/// Simplified CRC calculation (for testing only)
/// Real PNG uses CRC-32 with a specific polynomial
fn calculate_crc(data: &[u8]) -> u32 {
    // For testing purposes, use a simple hash
    // Real PNG CRC-32 would use polynomial 0xEDB88320
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    (hasher.finish() & 0xFFFFFFFF) as u32
}

/// Create a minimal test image (1x1 PNG) for simple tests
pub fn create_minimal_png() -> Vec<u8> {
    // Hardcoded minimal 1x1 PNG
    vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // 1x1 dimensions
        0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53, 0xDE, // IHDR data + CRC
        0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, // IDAT chunk
        0x08, 0xD7, 0x63, 0xF8, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, // IDAT data
        0x00, 0x18, 0xDD, 0x8D, 0x89, // IDAT CRC
        0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, // IEND chunk
        0xAE, 0x42, 0x60, 0x82, // IEND CRC
    ]
}

/// Create a test PDF file (minimal valid PDF)
pub fn create_test_pdf() -> Vec<u8> {
    // Minimal valid PDF structure
    let pdf_content = b"%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj
3 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >> >>
endobj
4 0 obj
<< /Length 44 >>
stream
BT
/F1 12 Tf
100 700 Td
(Test PDF) Tj
ET
endstream
endobj
xref
0 5
0000000000 65535 f 
0000000009 00000 n 
0000000058 00000 n 
0000000115 00000 n 
0000000254 00000 n 
trailer
<< /Size 5 /Root 1 0 R >>
startxref
353
%%EOF";

    pdf_content.to_vec()
}

/// Create a test video file (minimal MP4)
/// Note: Creating a real MP4 is complex, so we'll create a minimal valid structure
pub fn create_test_video() -> Vec<u8> {
    // Minimal MP4 structure (ftyp + mdat boxes)
    let mut mp4 = Vec::new();
    
    // ftyp box (file type)
    mp4.extend_from_slice(&[0x00, 0x00, 0x00, 0x20]); // Box size (32 bytes)
    mp4.extend_from_slice(b"ftyp");
    mp4.extend_from_slice(b"isom"); // Major brand
    mp4.extend_from_slice(&[0x00, 0x00, 0x02, 0x00]); // Minor version
    mp4.extend_from_slice(b"isom"); // Compatible brand
    mp4.extend_from_slice(b"iso2");
    mp4.extend_from_slice(b"mp41");
    
    // mdat box (media data - minimal)
    mp4.extend_from_slice(&[0x00, 0x00, 0x00, 0x08]); // Box size (8 bytes)
    mp4.extend_from_slice(b"mdat");
    
    mp4
}

/// Create a test audio file (minimal WAV)
pub fn create_test_audio() -> Vec<u8> {
    // Minimal WAV file structure
    let mut wav = Vec::new();
    
    // RIFF header
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&[0x24, 0x00, 0x00, 0x00]); // File size - 8
    wav.extend_from_slice(b"WAVE");
    
    // fmt chunk
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&[0x10, 0x00, 0x00, 0x00]); // fmt chunk size (16)
    wav.extend_from_slice(&[0x01, 0x00]); // Audio format (PCM)
    wav.extend_from_slice(&[0x01, 0x00]); // Num channels (1)
    wav.extend_from_slice(&[0x44, 0xAC, 0x00, 0x00]); // Sample rate (44100)
    wav.extend_from_slice(&[0x88, 0x58, 0x01, 0x00]); // Byte rate
    wav.extend_from_slice(&[0x02, 0x00]); // Block align
    wav.extend_from_slice(&[0x10, 0x00]); // Bits per sample (16)
    
    // data chunk
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // Data size (0 for minimal)
    
    wav
}

/// Create a file of specific size (for load testing)
pub fn create_file_of_size(size_bytes: usize, file_type: &str) -> Vec<u8> {
    match file_type {
        "image" | "png" => {
            // Create a PNG and pad it to desired size
            let mut png = create_minimal_png();
            if png.len() < size_bytes {
                png.extend(vec![0; size_bytes - png.len()]);
            }
            png.truncate(size_bytes);
            png
        }
        "pdf" => {
            let mut pdf = create_test_pdf();
            if pdf.len() < size_bytes {
                // Pad PDF with comments
                while pdf.len() < size_bytes {
                    pdf.extend(b"\n% Padding data for load testing\n");
                }
            }
            pdf.truncate(size_bytes);
            pdf
        }
        _ => {
            // Generic binary data
            vec![0; size_bytes]
        }
    }
}

/// Create multiple test files for batch operations
pub fn create_test_files(count: usize, file_type: &str) -> Vec<Vec<u8>> {
    (0..count)
        .map(|i| match file_type {
            "image" | "png" => create_test_png(50 + (i % 50), 50 + (i % 50)),
            "pdf" => create_test_pdf(),
            "video" => create_test_video(),
            "audio" => create_test_audio(),
            _ => create_minimal_png(),
        })
        .collect()
}
