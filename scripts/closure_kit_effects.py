"""SpellVisualKitEffectType.dbde + WoWDBDefs table foreign keys.

Sources: https://github.com/wowdev/WoWDBDefs/tree/master/definitions
https://github.com/wowdev/WoWDBDefs/blob/master/meta/enums/SpellVisualKitEffectType.dbde
Native baseline: godot/core/src/spell_visual/{kits,sounds}.rs.
Unknown discriminants/script semantics remain boundaries; numeric shader/animation
rows introduce no file request in the current native reader.
"""
from collections import deque

from closure_spell_seeds import value

EFFECT_TABLES = {
    1: 'SpellProceduralEffect', 2: 'SpellVisualKitModelAttach',
    3: 'CameraEffect', 4: 'CameraEffect', 6: 'SpellVisualAnim',
    7: 'ShadowyEffect', 8: 'SpellEffectEmission', 9: 'OutlineEffect',
    11: 'DissolveEffect', 12: 'EdgeGlowEffect', 13: 'BeamEffect',
    14: 'ClientSceneEffect', 15: 'CloneEffect', 16: 'GradientEffect',
    17: 'BarrageEffect', 18: 'RopeEffect', 19: 'SpellVisualScreenEffect',
    20: 'SpellVisualKitDecalAttach',
}
JOINS = {
    'SpellEffectEmission': {'AreaModelID': 'SpellVisualKitAreaModel'},
    'BeamEffect': {'BeamID': 'SpellChainEffects'},
    'DissolveEffect': {'TextureBlendSetID': 'TextureBlendSet'},
    'DecalProperties': {'TopTextureBlendSetID': 'TextureBlendSet',
                        'BotTextureBlendSetID': 'TextureBlendSet',
                        'CasterDecalPropertiesID': 'DecalProperties'},
    'SpellVisualKitDecalAttach': {'DecalPropertiesID': 'DecalProperties'},
    'SpellVisualScreenEffect': {'ScreenEffectID': 'ScreenEffect', 'ScreenEffectTypeID': 'ScreenEffectType'},
    'ScreenEffect': {'FullScreenEffectID': 'FullScreenEffect', 'SoundAmbienceID': 'SoundAmbience',
                     'ZoneMusicID': 'ZoneMusic', 'LightParamsID': 'LightParams'},
    'FullScreenEffect': {'TextureBlendSetID': 'TextureBlendSet'},
    'LightParams': {'LightSkyboxID': 'LightSkybox'},
}
FILE_FIELDS = {
    'SpellVisualKitAreaModel': {'ModelFileDataID': 'm2'},
    'TextureBlendSet': {'TextureFileDataID': 'blp'},
    'SpellChainEffects': {'TextureFileDataID': 'blp', 'TextureParticleFileDataID': 'blp'},
    'DecalProperties': {'FileDataID': 'blp', 'Field_11_2_0_61476_024': 'blp'},
    'FullScreenEffect': {'OverlayTextureFileDataID': 'blp'},
    'LightSkybox': {'SkyboxFileDataID': 'm2'},
}
SOUND_FIELDS = {'SpellChainEffects': {'SoundKitID'}, 'ZoneMusic': {'Sounds'},
                'SoundAmbience': {'AmbienceID', 'AmbienceStartID', 'AmbienceStopID', 'SoundKitID'}}
KIT_FIELDS = {'CloneEffect': {'StartSpellVisualKitID', 'StateSpellVisualKitID'},
              'SpellVisualKitDecalAttach': {'SpellVisualKitID'}}


def base_field(field):
    head, _, suffix = field.rpartition('_')
    return head if suffix.isdecimal() else field


class KitEffectReferences:
    def __init__(self, graph, catalogs, effects, sounds, kits):
        self.graph, self.catalogs = graph, catalogs
        self.effects, self.sounds, self.kits = effects, sounds, kits
        self.indices, self.visited = {}, set()

    def seed(self, kind, effect, reason):
        if kind == 5:
            self.sounds.add(effect)
        elif kind == 10:
            self.seed_unit_sounds()
        elif kind in EFFECT_TABLES:
            self.walk(EFFECT_TABLES[kind], effect)
        else:
            self.graph.issue('unsupported_spell_kit_effect', reason)

    def seed_unit_sounds(self):
        # The event's unit is conditional; enumerate every authored voice, not a
        # guessed type-10 -> SoundKit mapping (native spell_visual_voice.rs).
        for row in self.catalogs.rows('CreatureSoundData'):
            for field in row:
                if field != 'ID' and ('Sound' in field or field.startswith('Fidget')):
                    self.sounds.add(value(row, field))

    def walk(self, table, identity):
        pending = deque([(table, identity)])
        while pending:
            table, identity = pending.popleft()
            if not identity or (table, identity) in self.visited:
                continue
            self.visited.add((table, identity))
            if table not in self.indices:
                self.indices[table] = self.catalogs.index(table)
            row = self.indices[table].get(identity)
            if row is None:
                self.graph.issue('missing_metadata_row', f'{table} ID={identity}')
                continue
            self.join_row(table, identity, row, pending)

    def join_row(self, table, identity, row, pending):
        asset_edges = 0
        for field in row:
            exact = (field in JOINS.get(table, {}) or field in FILE_FIELDS.get(table, {})
                     or field in SOUND_FIELDS.get(table, set()) or field in KIT_FIELDS.get(table, set()))
            base = field if exact else base_field(field)
            relevant = (base in JOINS.get(table, {}) or base in FILE_FIELDS.get(table, {})
                        or base in SOUND_FIELDS.get(table, set()) or base in KIT_FIELDS.get(table, set())
                        or (table == 'SpellChainEffects' and base == 'SpellChainEffectID')
                        or (table in {'SpellVisualKitModelAttach', 'BarrageEffect'} and base == 'SpellVisualEffectNameID'))
            if not relevant:
                continue
            number = value(row, field)
            if not number:
                continue
            target = JOINS.get(table, {}).get(base)
            if table == 'SpellChainEffects' and base == 'SpellChainEffectID':
                target = table
            if target:
                pending.append((target, number))
                asset_edges += 1
            kind = FILE_FIELDS.get(table, {}).get(base)
            if kind:
                if number > 0:
                    self.graph.add(number, kind, f'{table} {identity} {field}')
                else:
                    self.graph.issue('unsupported_spell_procedural_model', f'{table} {identity} {field}={number}')
                asset_edges += 1
            if base in SOUND_FIELDS.get(table, set()):
                self.sounds.add(number)
                asset_edges += 1
            if base in KIT_FIELDS.get(table, set()):
                self.kits.add(number)
                asset_edges += 1
            if base == 'SpellVisualEffectNameID' and table in {'SpellVisualKitModelAttach', 'BarrageEffect'}:
                self.effects.add(number)
                asset_edges += 1
        if table == 'ClientSceneEffect':
            self.graph.issue('client_scene_asset_edges', f'ClientSceneEffect {identity} SceneScriptPackageID={row.get("SceneScriptPackageID")}; script asset semantics not parsed')
        elif not asset_edges:
            self.graph.resolve('spell_kit_effect', None, 'not_needed',
                               f'{table} {identity}: hashed DB2 row; no authored file join; numeric shader/animation parameters; native kits.rs makes no file request')
