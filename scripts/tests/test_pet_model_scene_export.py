import importlib.util
from pathlib import Path
import unittest
import sys
sys.path.insert(0, str(Path(__file__).parents[1]))

SPEC = importlib.util.spec_from_file_location('pet_scene_export', Path(__file__).parents[1] / 'export_pet_model_scenes.py')
EXPORT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(EXPORT)

class PetModelSceneExportTest(unittest.TestCase):
    def sources(self):
        return {
            'UiModelScene': [{'ID': 6, 'UiSystemType': 1}],
            'UiModelSceneCamera': [{'ID': 3, 'UiModelSceneID': 6, 'CameraType': 0, 'ScriptTag': 'primary',
                'Target_0': 0., 'Target_1': 0., 'Target_2': 0., 'Yaw': 2.6179938316, 'Pitch': 0., 'Roll': 0., 'ZoomDistance': 14.}],
            'UiModelSceneActor': [{'ID': 5, 'UiModelSceneID': 6, 'ScriptTag': 'unwrapped', 'Flags': 9,
                'Position_0': 0., 'Position_1': 0., 'Position_2': 0., 'OrientationYaw': 0., 'OrientationPitch': 0., 'OrientationRoll': 0.,
                'UiModelSceneActorDisplayID': 245, 'NormalizedScale': 1.}],
            'UiModelSceneActorDisplay': [{'ID': 245, 'Scale': 7.}],
        }

    def test_source_camera_and_actor_scale_are_exported_without_mesh_fit_defaults(self):
        scenes = EXPORT.build_scenes(self.sources(), {6})
        self.assertEqual(scenes, [{'id': 6, 'target': [0., 0., 0.], 'yaw': 2.6179938316, 'pitch': 0., 'roll': 0.,
            'zoom_distance': 14., 'actor_position': [0., 0., 0.], 'actor_orientation': [0., 0., 0.],
            'origin_flags': 9, 'normalized_scale': 1., 'scale': 7.}])

    def test_missing_actor_display_is_explicit_not_default_scale(self):
        tables = self.sources()
        tables['UiModelSceneActorDisplay'] = []
        with self.assertRaisesRegex(ValueError, 'scene 6.*245'):
            EXPORT.build_scenes(tables, {6})

if __name__ == '__main__':
    unittest.main()
