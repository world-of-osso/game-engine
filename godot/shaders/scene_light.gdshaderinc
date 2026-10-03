// RetailSceneLight, in authored sRGB-byte / 255 space. Like WebWowViewerCpp's per-scene
// SceneWideParams.extLight (commonLightFunctions.slang:26-52, filled once per frame by
// MapSceneRenderer), the scene's light is a set of global uniforms written once per light
// change (lighting/mod.rs `TerrainLight::bind_scene`); every material that `bind_model`
// marks `scene_light` reads them. Other materials (character previews, quest markers)
// keep their own values.
uniform bool scene_light = false;
global uniform vec3 scene_ambient;
global uniform vec3 scene_horizon_ambient;
global uniform vec3 scene_ground_ambient;
global uniform vec3 scene_direct;
// The world direction sunlight travels, not the direction towards the sun.
global uniform vec3 scene_sun_direction;

uniform vec3 ambient = vec3(1.0);
uniform vec3 horizon_ambient = vec3(1.0);
uniform vec3 ground_ambient = vec3(1.0);
uniform vec3 direct;
uniform vec3 sun_direction = vec3(0.0, -1.0, 0.0);

vec3 light_ambient() {
	return scene_light ? scene_ambient : ambient;
}

vec3 light_horizon_ambient() {
	return scene_light ? scene_horizon_ambient : horizon_ambient;
}

vec3 light_ground_ambient() {
	return scene_light ? scene_ground_ambient : ground_ambient;
}

vec3 light_direct() {
	return scene_light ? scene_direct : direct;
}

vec3 light_sun_direction() {
	return scene_light ? scene_sun_direction : sun_direction;
}
