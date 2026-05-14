use resvg::tiny_skia::{PixmapMut, Transform};

const VOLUME_ON_SVG: &[u8] = include_bytes!("assets/volume-2.svg");
const VOLUME_MUTED_SVG: &[u8] = include_bytes!("assets/volume-x.svg");
const INDICATOR_BACKGROUND_ON_SVG: &[u8] = include_bytes!("assets/indicator-background-2.svg");
const INDICATOR_BACKGROUND_MUTED_SVG: &[u8] = include_bytes!("assets/indicator-background-x.svg");
const SVG_VIEWBOX_SIZE: f32 = 24.0;
const TRAY_ICON_SCALE: f32 = 0.58;
const MAX_CHANNEL_VALUE: u16 = 255;
const TRANSPARENT_PIXEL: [u8; 4] = [0, 0, 0, 0];

#[derive(Clone, Copy, Debug)]
pub struct IndicatorState {
    pub muted: bool,
    pub background_transparency: u8,
    pub foreground_transparency: u8,
}

pub fn render_indicator_rgba(size: u32, state: IndicatorState) -> Vec<u8> {
    let mut frame = vec![0; frame_len(size)];
    draw_indicator(&mut frame, size, state);
    frame
}

#[cfg(not(windows))]
pub fn render_tray_icon_rgba(size: u32) -> Vec<u8> {
    let mut frame = vec![0; frame_len(size)];
    let Some(mut pixmap) = PixmapMut::from_bytes(&mut frame, size, size) else {
        return frame;
    };
    draw_svg_icon(&mut pixmap, size as f32, false);
    unpremultiply_frame(&mut frame);
    frame
}

fn frame_len(size: u32) -> usize {
    (size as usize)
        .checked_mul(size as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .expect("indicator image size overflows usize")
}

fn draw_indicator(frame: &mut [u8], size: u32, state: IndicatorState) {
    for pixel in frame.chunks_exact_mut(4) {
        pixel.copy_from_slice(&TRANSPARENT_PIXEL);
    }

    let Some(mut pixmap) = PixmapMut::from_bytes(frame, size, size) else {
        return;
    };

    if size <= 1 {
        return;
    }

    draw_indicator_background_svg(&mut pixmap, size as f32, state.muted);
    apply_alpha(frame, state.background_transparency);
    draw_svg_icon_with_alpha(frame, size, state.muted, state.foreground_transparency);
    unpremultiply_frame(frame);
}

fn draw_indicator_background_svg(pixmap: &mut PixmapMut<'_>, size: f32, muted: bool) {
    let svg = if muted {
        INDICATOR_BACKGROUND_MUTED_SVG
    } else {
        INDICATOR_BACKGROUND_ON_SVG
    };
    draw_svg(
        pixmap,
        svg,
        Transform::from_scale(size / SVG_VIEWBOX_SIZE, size / SVG_VIEWBOX_SIZE),
    );
}

fn draw_svg_icon_with_alpha(frame: &mut [u8], size: u32, muted: bool, alpha: u8) {
    let mut layer = vec![0; frame.len()];
    let Some(mut pixmap) = PixmapMut::from_bytes(&mut layer, size, size) else {
        return;
    };
    draw_svg_icon(&mut pixmap, size as f32, muted);
    apply_alpha(&mut layer, alpha);
    composite_layer(frame, &layer);
}

fn draw_svg_icon(pixmap: &mut PixmapMut<'_>, size: f32, muted: bool) {
    let svg = if muted {
        VOLUME_MUTED_SVG
    } else {
        VOLUME_ON_SVG
    };

    let icon_size = size * TRAY_ICON_SCALE;
    let offset = (size - icon_size) / 2.0;
    let scale = icon_size / SVG_VIEWBOX_SIZE;
    draw_svg(
        pixmap,
        svg,
        Transform::from_row(scale, 0.0, 0.0, scale, offset, offset),
    );
}

fn draw_svg(pixmap: &mut PixmapMut<'_>, svg: &[u8], transform: Transform) {
    let Ok(tree) = resvg::usvg::Tree::from_data(svg, &resvg::usvg::Options::default()) else {
        return;
    };

    resvg::render(&tree, transform, pixmap);
}

fn apply_alpha(frame: &mut [u8], alpha: u8) {
    for pixel in frame.chunks_exact_mut(4) {
        for channel in pixel {
            *channel = ((*channel as u16 * alpha as u16) / MAX_CHANNEL_VALUE) as u8;
        }
    }
}

fn composite_layer(frame: &mut [u8], layer: &[u8]) {
    for (dst, src) in frame.chunks_exact_mut(4).zip(layer.chunks_exact(4)) {
        let src_alpha = src[3] as u16;
        let inverse_src_alpha = MAX_CHANNEL_VALUE - src_alpha;
        for channel in 0..3 {
            dst[channel] = (src[channel] as u16
                + (dst[channel] as u16 * inverse_src_alpha) / MAX_CHANNEL_VALUE)
                as u8;
        }
        dst[3] = (src_alpha + (dst[3] as u16 * inverse_src_alpha) / MAX_CHANNEL_VALUE) as u8;
    }
}

fn unpremultiply_frame(frame: &mut [u8]) {
    for pixel in frame.chunks_exact_mut(4) {
        let alpha = pixel[3] as u16;
        if alpha == 0 || alpha == MAX_CHANNEL_VALUE {
            continue;
        }
        pixel[0] = ((pixel[0] as u16 * MAX_CHANNEL_VALUE) / alpha).min(MAX_CHANNEL_VALUE) as u8;
        pixel[1] = ((pixel[1] as u16 * MAX_CHANNEL_VALUE) / alpha).min(MAX_CHANNEL_VALUE) as u8;
        pixel[2] = ((pixel[2] as u16 * MAX_CHANNEL_VALUE) / alpha).min(MAX_CHANNEL_VALUE) as u8;
    }
}
