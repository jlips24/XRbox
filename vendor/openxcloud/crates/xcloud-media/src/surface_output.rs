//! Output policy for opaque surface buffers, whose byte size is not CPU pixels.
/// Codec-config buffers and an empty EOS marker are not pictures. Do not gate
/// ordinary opaque surface frames on a nonzero CPU byte count.
pub fn render_surface_output(size: i32, flags: u32) -> bool {
    flags & 2 == 0 && !(flags & 4 != 0 && size == 0)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opaque_frames_are_rendered_while_config_and_empty_eos_are_not() {
        assert!(render_surface_output(0, 0));
        assert!(render_surface_output(1024, 0));
        assert!(!render_surface_output(128, 2));
        assert!(!render_surface_output(0, 4));
        assert!(render_surface_output(1024, 4));
    }
}
