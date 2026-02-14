use std::path::Path;

use anyhow::{anyhow, Result};
use ffmpeg_next::codec;
use ffmpeg_next::format::Pixel;
use ffmpeg_next::software::scaling;
use ffmpeg_next::Rational;

pub fn join_frames(input_dir: &Path, output_path: &Path, fps: u32, bitrate: usize) -> Result<()> {
    ffmpeg_next::init()?;

    let mut frames: Vec<std::path::PathBuf> = std::fs::read_dir(input_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map_or(false, |ext| ext == "png"))
        .collect();
    frames.sort();

    if frames.is_empty() {
        return Err(anyhow!("No PNG frames found in {:?}", input_dir));
    }

    let first_img = image::open(&frames[0])?;
    let width = first_img.width();
    let height = first_img.height();
    let total = frames.len();
    println!(
        "[join] {} frames ({}x{}) @ {}fps, bitrate {}",
        total, width, height, fps, bitrate
    );

    let mut octx = ffmpeg_next::format::output(output_path)?;

    let codec = ffmpeg_next::encoder::find(codec::Id::VP9)
        .ok_or_else(|| anyhow!("VP9 encoder (libvpx-vp9) not found"))?;

    let mut ost = octx.add_stream(Some(codec))?;
    let stream_index = ost.index();

    let mut enc = codec::context::Context::new_with_codec(codec)
        .encoder()
        .video()?;

    enc.set_width(width);
    enc.set_height(height);
    enc.set_format(Pixel::YUVA420P);
    enc.set_time_base(Rational::new(1, fps as i32));
    enc.set_frame_rate(Some(Rational::new(fps as i32, 1)));
    enc.set_bit_rate(bitrate);

    let mut opts = ffmpeg_next::Dictionary::new();
    opts.set("auto-alt-ref", "0");

    let mut enc = enc.open_with(opts)?;
    ost.set_parameters(codec::Parameters::from(&enc));

    octx.write_header()?;

    let mut scaler = scaling::Context::get(
        Pixel::RGBA,
        width,
        height,
        Pixel::YUVA420P,
        width,
        height,
        scaling::Flags::BILINEAR,
    )?;

    let ost_time_base = octx.stream(stream_index).unwrap().time_base();
    let enc_time_base = Rational::new(1, fps as i32);

    for (idx, path) in frames.iter().enumerate() {
        let img = image::open(path)?;
        let rgba = img.to_rgba8();
        let raw = rgba.as_raw();

        let mut rgba_frame = ffmpeg_next::frame::Video::new(Pixel::RGBA, width, height);
        let stride = rgba_frame.stride(0);
        for y in 0..height as usize {
            let src_start = y * (width as usize * 4);
            let dst_start = y * stride;
            rgba_frame.data_mut(0)[dst_start..dst_start + width as usize * 4]
                .copy_from_slice(&raw[src_start..src_start + width as usize * 4]);
        }

        let mut yuva_frame = ffmpeg_next::frame::Video::empty();
        scaler.run(&rgba_frame, &mut yuva_frame)?;
        yuva_frame.set_pts(Some(idx as i64));

        enc.send_frame(&yuva_frame)?;
        receive_and_write(
            &mut enc,
            &mut octx,
            stream_index,
            enc_time_base,
            ost_time_base,
        )?;

        if (idx + 1) % 10 == 0 || idx + 1 == total {
            println!("[join] [{}/{}]", idx + 1, total);
        }
    }

    enc.send_eof()?;
    receive_and_write(
        &mut enc,
        &mut octx,
        stream_index,
        enc_time_base,
        ost_time_base,
    )?;

    octx.write_trailer()?;

    let file_size = std::fs::metadata(output_path).map(|m| m.len()).unwrap_or(0);
    println!(
        "[join] Done! {} → {:.1}MB",
        output_path.display(),
        file_size as f64 / 1_048_576.0
    );
    Ok(())
}

fn receive_and_write(
    enc: &mut ffmpeg_next::encoder::video::Encoder,
    octx: &mut ffmpeg_next::format::context::Output,
    stream_index: usize,
    enc_tb: Rational,
    ost_tb: Rational,
) -> Result<()> {
    let mut packet = ffmpeg_next::Packet::empty();
    while enc.receive_packet(&mut packet).is_ok() {
        packet.set_stream(stream_index);
        packet.rescale_ts(enc_tb, ost_tb);
        packet.write_interleaved(octx)?;
    }
    Ok(())
}
