shader_type spatial;
render_mode unshaded, cull_disabled;
uniform samplerExternalOES video : source_color;
uniform bool flip = false;
void fragment() {
    vec2 uv = UV;
    if (flip) { uv.y = 1.0 - uv.y; }
    ALBEDO = texture(video, uv).rgb;
}
