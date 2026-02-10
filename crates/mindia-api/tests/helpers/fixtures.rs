//! Test fixtures: minimal PNG/PDF/video/audio blobs.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Minimal valid 1x1 PNG bytes.
pub fn create_minimal_png() -> Vec<u8> {
    vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90,
        0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8,
        0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0x89, 0x00, 0x00, 0x00,
        0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ]
}

/// Create a minimal valid PNG of given dimensions (simplified).
pub fn create_test_png(width: u32, height: u32) -> Vec<u8> {
    let w = width.min(65535);
    let h = height.min(65535);
    let mut png = Vec::new();
    png.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
    png.extend_from_slice(&13u32.to_be_bytes());
    png.extend_from_slice(b"IHDR");
    png.extend_from_slice(&w.to_be_bytes());
    png.extend_from_slice(&h.to_be_bytes());
    png.extend_from_slice(&[8, 2, 0, 0, 0]);
    let mut hasher = DefaultHasher::new();
    png[png.len() - 17..].hash(&mut hasher);
    png.extend_from_slice(&(hasher.finish() as u32 & 0xFFFF_FFFF).to_be_bytes());
    let idat_data = vec![0u8; (w * h * 3) as usize];
    png.extend_from_slice(&(idat_data.len() as u32).to_be_bytes());
    png.extend_from_slice(b"IDAT");
    png.extend_from_slice(&idat_data);
    png.extend_from_slice(&[0, 0, 0, 0]);
    png.extend_from_slice(b"IEND");
    png.extend_from_slice(&0xAE426082u32.to_be_bytes());
    png
}

/// Minimal valid PDF.
pub fn create_test_pdf() -> Vec<u8> {
    b"%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj
3 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>
endobj
xref
0 4
0000000000 65535 f 
0000000009 00000 n 
0000000058 00000 n 
0000000115 00000 n 
trailer
<< /Size 4 /Root 1 0 R >>
startxref
200
%%EOF"
        .to_vec()
}

/// Minimal MP4 (ftyp + mdat).
pub fn create_test_video() -> Vec<u8> {
    let mut mp4 = Vec::new();
    mp4.extend_from_slice(&[0x00, 0x00, 0x00, 0x20]);
    mp4.extend_from_slice(b"ftyp");
    mp4.extend_from_slice(b"isom");
    mp4.extend_from_slice(&[0x00, 0x00, 0x02, 0x00]);
    mp4.extend_from_slice(b"isomiso2mp41");
    mp4.extend_from_slice(&[0x00, 0x00, 0x00, 0x08]);
    mp4.extend_from_slice(b"mdat");
    mp4
}
