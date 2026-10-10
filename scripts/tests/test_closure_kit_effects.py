"""Concrete kit effect joins; cycles, absent rows, and unknown types stay explicit."""
from pathlib import Path
import sys
import tempfile
import unittest
sys.path.insert(0, str(Path(__file__).parents[1]))
from asset_closure import Closure
from closure_seeds import Catalogs
from closure_spell_seeds import seed_spell_visuals


class KitEffectsTests(unittest.TestCase):
    def test_effect_assets_recursive_chain_and_shader_only_rows(self):
        tables = {
            'SpellMisc': 'ID,SpellID\n',
            'SpellXSpellVisual': 'ID,SpellID,SpellVisualID\n1,100,10\n',
            'SpellVisual': 'ID\n10\n',
            'SpellVisualEvent': 'ID,SpellVisualID,SpellVisualKitID\n1,10,30\n',
            'SpellVisualKit': 'ID\n30\n',
            'SpellVisualKitModelAttach': 'ID\n',
            'SpellVisualMissile': 'ID\n',
            'SpellVisualEffectName': 'ID,ModelFileDataID\n44,501\n',
            'SpellVisualKitEffect': 'ID,ParentSpellVisualKitID,EffectType,Effect\n1,30,13,40\n2,30,11,41\n3,30,8,42\n4,30,17,43\n5,30,7,45\n6,30,7,999\n7,30,99,1\n',
            'BeamEffect': 'ID,BeamID\n40,50\n',
            'SpellChainEffects': 'ID,TextureFileDataID_0,TextureParticleFileDataID,SoundKitID,SpellChainEffectID_0\n50,601,602,70,51\n51,603,0,0,50\n',
            'DissolveEffect': 'ID,TextureBlendSetID\n41,60\n',
            'TextureBlendSet': 'ID,TextureFileDataID_0,TextureFileDataID_1\n60,604,605\n',
            'SpellEffectEmission': 'ID,AreaModelID\n42,61\n',
            'SpellVisualKitAreaModel': 'ID,ModelFileDataID\n61,500\n',
            'BarrageEffect': 'ID,SpellVisualEffectNameID\n43,44\n',
            'ShadowyEffect': 'ID\n45\n',
            'SoundKitEntry': 'ID,SoundKitID,FileDataID\n1,70,700\n',
        }
        with tempfile.TemporaryDirectory() as tmp:
            data=Path(tmp); root=data/'db2/fixture'; root.mkdir(parents=True)
            for name,text in tables.items(): (root/(name+'.csv')).write_text(text)
            graph=Closure(data,{700:'sound/beam.ogg'},'wow','fixture')
            seed_spell_visuals(graph,Catalogs(graph),[100])
            self.assertEqual(sorted(graph.assets),[(500,'m2'),(501,'m2'),(601,'blp'),(602,'blp'),(603,'blp'),(604,'blp'),(605,'blp'),(700,'ogg')])
            self.assertTrue(any(c=='missing_metadata_row' and r=='ShadowyEffect ID=999' for c,r,_ in graph.unresolved))
            self.assertEqual([r for c,r,_ in graph.unresolved if c=='unsupported_spell_kit_effect'],['kit 30 effect type 99 effect 1'])
            self.assertTrue(any(c=='spell_kit_effect' and status=='not_needed' and 'ShadowyEffect 45' in evidence for c,_,status,evidence in graph.resolved))


if __name__=='__main__': unittest.main()
