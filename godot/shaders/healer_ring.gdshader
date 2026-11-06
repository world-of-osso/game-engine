shader_type spatial;
// Healer ground ring band. The mesh is a draped annulus that spans a little past the
// band, so both edges are analytic here: the radial distance gives a one-pixel inner edge
// and a soft outer falloff, and the interpolated angle fraction gives an anti-aliased arc
// start. Depth-tested and depth-write-free, so world geometry occludes the ring like a
// ground decal.
render_mode unshaded, cull_disabled, blend_mix, depth_draw_never, shadows_disabled;

uniform vec4 ring_color : source_color = vec4(1.0);
uniform float inner_radius = 0.82;
uniform float outer_radius = 1.12;
// Radians clockwise from 12 o'clock where the drawn arc begins. The arc always ends at
// 12 o'clock, so the gap grows clockwise from there as health drops.
uniform float arc_start = 0.0;
uniform float alpha = 1.0;
// Yards of soft falloff at the outer edge.
uniform float outer_softness = 0.035;

varying vec2 local_xz;
// Station fraction around the ring, 0 at 12 o'clock and 1 back at 12 o'clock. Interpolating
// a fraction avoids `atan` and its branch cut at the seam.
varying float angle_fraction;

void vertex() {
	local_xz = VERTEX.xz;
	angle_fraction = UV.y;
}

void fragment() {
	float radius = length(local_xz);
	float aa = max(fwidth(radius) * 1.2, 0.0025);
	float inner_edge = smoothstep(inner_radius - aa, inner_radius + aa, radius);
	float outer_edge = 1.0 - smoothstep(outer_radius - outer_softness, outer_radius, radius);

	float start = arc_start / 6.28318530718;
	float arc_aa = max(fwidth(angle_fraction) * 1.2, 0.0015);
	float arc = smoothstep(start - arc_aa, start + arc_aa, angle_fraction);

	ALBEDO = ring_color.rgb;
	ALPHA = ring_color.a * alpha * inner_edge * outer_edge * arc;
	if (ALPHA <= 0.002) {
		discard;
	}
}
