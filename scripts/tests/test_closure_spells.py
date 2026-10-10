"""Reachable spell roots traverse visual kits, missiles, audio and textures."""
from pathlib import Path
import hashlib
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).parents[1]))
from asset_closure import Closure
from closure_seeds import Catalogs


class SpellRootsTests(unittest.TestCase):
    def test_spell_visual_chain_and_authored_kit_cycle(self):
        from closure_spell_seeds import seed_spell_visuals
        tables = {
            'SpellMisc': 'ID,SpellID,DifficultyID,SpellIconFileDataID\n1,100,7,72\n',
            'SpellXSpellVisual': 'ID,SpellID,SpellVisualID,SpellIconFileID\n1,100,10,71\n2,999,99,90\n',
            'SpellVisual': 'ID,SpellVisualMissileSetID\n10,20\n99,99\n',
            'SpellVisualEvent': 'ID,SpellVisualID,SpellVisualKitID\n1,10,30\n',
            'SpellVisualKit': 'ID,FallbackSpellVisualKitID\n30,31\n31,30\n',
            'SpellVisualKitEffect': 'ID,ParentSpellVisualKitID,EffectType,Effect\n1,31,5,50\n2,30,99,88\n3,30,2,10\n',
            'SpellVisualKitModelAttach': 'ID,ParentSpellVisualKitID,SpellVisualEffectNameID\n1,30,40\n10,999,42\n',
            'SpellVisualMissile': 'ID,SpellVisualMissileSetID,SpellVisualEffectNameID,SoundEntriesID\n1,20,41,51\n',
            'SpellVisualEffectName': 'ID,ModelFileDataID,TextureFileDataID\n40,60,70\n41,61,0\n42,62,0\n',
            'SoundKitEntry': 'ID,SoundKitID,FileDataID\n1,50,80\n2,51,81\n3,99,90\n4,50,82\n',
        }
        with tempfile.TemporaryDirectory() as tmp:
            data = Path(tmp)
            root = data / 'db2/fixture'
            root.mkdir(parents=True)
            for name, text in tables.items():
                (root / (name + '.csv')).write_text(text)
            (data / 'sounds/spells').mkdir(parents=True)
            (data / 'sounds/spells/80.ogg').write_bytes(b'OggSfixture')
            graph = Closure(data, {80: 'sound/a.ogg', 81: 'sound/b.mp3'}, 'wow', 'fixture')
            seed_spell_visuals(graph, Catalogs(graph), [100])
            self.assertEqual(sorted(graph.assets), [(60, 'm2'), (61, 'm2'), (62, 'm2'), (70, 'blp'), (71, 'blp'), (72, 'blp'), (80, 'ogg'), (81, 'mp3'), (82, 'audio')])
            self.assertTrue(any(code == 'unsupported_spell_kit_effect' and '99' in reason for code, reason, _ in graph.unresolved))
            self.assertTrue(any(code == 'unknown_sound_file_type' and fdid == 82 for code, _, fdid in graph.unresolved))
            result = graph.run()
            self.assertEqual(next(row for row in result['assets'] if row['fdid'] == 80)['present_files'],
                             [{'path': 'sounds/spells/80.ogg', 'size': 11, 'sha256': hashlib.sha256(b'OggSfixture').hexdigest()}])


if __name__ == '__main__':
    unittest.main()
