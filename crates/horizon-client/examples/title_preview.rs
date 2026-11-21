//! Offscreen software rasterizer for previewing the Starling Title Screen without a display.
//! Usage: cargo run --profile fast -p horizon-client-lib --example title_preview -- [out.png]

extern crate horizon_client_lib as horizon_client;

use horizon_client::{
    hud::{HudAtlas, HudBatch},
    title::Title,
};
use std::{
    fs::File,
    io::BufWriter,
    path::{Path, PathBuf},
};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let out_path = args
        .first()
        .map(|s| s.as_str())
        .unwrap_or("title_preview.png");

    let width: usize = 854;
    let height: usize = 480;

    // Load atlas with graceful fallback to built-in boot atlas
    let pack_dir = Path::new("packaging/lowbudget/pack");
    let atlas = HudAtlas::build(pack_dir).unwrap_or_else(|_| HudAtlas::boot());

    let mut batch = HudBatch::new([width as u32, height as u32]);
    let title = Title::new(PathBuf::new(), pack_dir);

    // Draw the title screen layout into HudBatch
    title.draw(&atlas, &mut batch, (0.0, 0.0), false);

    // 1. Render background: Complementary Reimagined dawn atmosphere (time 23400)
    let mut pixels = vec![0u8; width * height * 4];
    for y in 0..height {
        let v = y as f32 / height as f32;
        // Deep twilight indigo overhead (y = 0) to warm pre-dawn golden horizon (y = height)
        let r = (24.0 * (1.0 - v) + 242.0 * v).clamp(0.0, 255.0) as u8;
        let g = (28.0 * (1.0 - v) + 162.0 * v).clamp(0.0, 255.0) as u8;
        let b = (54.0 * (1.0 - v) + 76.0 * v).clamp(0.0, 255.0) as u8;
        for x in 0..width {
            let idx = (y * width + x) * 4;
            pixels[idx] = r;
            pixels[idx + 1] = g;
            pixels[idx + 2] = b;
            pixels[idx + 3] = 255;
        }
    }

    // 2. Rasterize HudBatch geometry over the background
    let aw = atlas.width as usize;
    let ah = atlas.height as usize;
    let atlas_pixels = atlas.pixels();
    let has_atlas = !atlas_pixels.is_empty() && aw > 0 && ah > 0;
    let scale = batch.scale;

    for quad in batch.vertices.chunks_exact(6) {
        let x0 = quad[0].pos[0].min(quad[2].pos[0]) * scale;
        let x1 = quad[0].pos[0].max(quad[2].pos[0]) * scale;
        let y0 = quad[0].pos[1].min(quad[2].pos[1]) * scale;
        let y1 = quad[0].pos[1].max(quad[2].pos[1]) * scale;

        let col = quad[0].color;
        let vert_r = (col & 0xFF) as f32 / 255.0;
        let vert_g = ((col >> 8) & 0xFF) as f32 / 255.0;
        let vert_b = ((col >> 16) & 0xFF) as f32 / 255.0;
        let vert_a = ((col >> 24) & 0xFF) as f32 / 255.0;

        let uv0 = quad[0].uv;
        let uv2 = quad[2].uv;
        let is_textured = has_atlas && (uv0[0] != uv2[0] || uv0[1] != uv2[1]);

        let px0 = (x0.round() as usize).clamp(0, width);
        let px1 = (x1.round() as usize).clamp(0, width);
        let py0 = (y0.round() as usize).clamp(0, height);
        let py1 = (y1.round() as usize).clamp(0, height);

        if px1 <= px0 || py1 <= py0 {
            continue;
        }

        let qw = (x1 - x0).max(1.0);
        let qh = (y1 - y0).max(1.0);

        for py in py0..py1 {
            let t = ((py as f32 - y0) / qh).clamp(0.0, 1.0);
            for px in px0..px1 {
                let s = ((px as f32 - x0) / qw).clamp(0.0, 1.0);

                let (src_r, src_g, src_b, src_a) = if is_textured {
                    let u = (uv0[0] + s * (uv2[0] - uv0[0])).clamp(0.0, 1.0);
                    let v = (uv0[1] + t * (uv2[1] - uv0[1])).clamp(0.0, 1.0);
                    let tx = ((u * (aw - 1) as f32).round() as usize).min(aw - 1);
                    let ty = ((v * (ah - 1) as f32).round() as usize).min(ah - 1);
                    let tidx = (ty * aw + tx) * 4;

                    let tr = atlas_pixels[tidx] as f32 / 255.0;
                    let tg = atlas_pixels[tidx + 1] as f32 / 255.0;
                    let tb = atlas_pixels[tidx + 2] as f32 / 255.0;
                    let ta = atlas_pixels[tidx + 3] as f32 / 255.0;

                    (vert_r * tr, vert_g * tg, vert_b * tb, vert_a * ta)
                } else {
                    (vert_r, vert_g, vert_b, vert_a)
                };

                if src_a < 0.01 {
                    continue;
                }

                let idx = (py * width + px) * 4;
                let bg_r = pixels[idx] as f32 / 255.0;
                let bg_g = pixels[idx + 1] as f32 / 255.0;
                let bg_b = pixels[idx + 2] as f32 / 255.0;

                let out_r = src_r * src_a + bg_r * (1.0 - src_a);
                let out_g = src_g * src_a + bg_g * (1.0 - src_a);
                let out_b = src_b * src_a + bg_b * (1.0 - src_a);

                pixels[idx] = (out_r * 255.0).round().clamp(0.0, 255.0) as u8;
                pixels[idx + 1] = (out_g * 255.0).round().clamp(0.0, 255.0) as u8;
                pixels[idx + 2] = (out_b * 255.0).round().clamp(0.0, 255.0) as u8;
                pixels[idx + 3] = 255;
            }
        }
    }

    // Write PNG
    let file = File::create(out_path).expect("failed to create preview PNG");
    let mut encoder = png::Encoder::new(BufWriter::new(file), width as u32, height as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("write header");
    writer.write_image_data(&pixels).expect("write pixels");

    println!("Saved preview to {}", out_path);
}
