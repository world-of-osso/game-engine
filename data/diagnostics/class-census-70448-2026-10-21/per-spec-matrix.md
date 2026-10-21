# Per-spec matrix —70448,2026-10-21

Fresh native base-probe receipts only. PASS is not a full ability/script/proc/rotation/UDP certificate. FAIL includes partial dispatch and unqualified fixture conditions, not only product defects. The current pipeline treats Aura4/226 as Script kind without requiring a registered hook (auras/handlers.rs), and admits DUMMY3 targets without a hook (spell_cast/mod.rs); silent missing hooks are NOT exhaustively detected by this probe. NONE0 warnings are retained as context, not generic gaps. Knowledge-only Parry22/Block23/DualWield40/TitansGrip155 passives need no aura. Historical analyzer statuses are not reused.

|Class / spec|PASS|FAIL|a|b|c|d|Total|
|---|---:|---:|---:|---:|---:|---:|---:|---:|
|Mage / Arcane (62)|192|43|30|0|0|13|235|
|Mage / Fire (63)|190|44|30|0|0|14|234|
|Mage / Frost (64)|194|40|29|0|0|11|234|
|Paladin / Holy (65)|204|21|16|0|0|5|225|
|Paladin / Protection (66)|197|24|17|0|0|7|221|
|Paladin / Retribution (70)|166|24|19|0|0|5|190|
|Warrior / Arms (71)|147|15|5|0|0|10|162|
|Warrior / Fury (72)|152|14|5|0|0|9|166|
|Warrior / Protection (73)|156|15|8|0|0|7|171|
|Druid / Balance (102)|225|25|11|0|0|14|250|
|Druid / Feral (103)|203|26|9|0|0|17|229|
|Druid / Guardian (104)|222|29|15|0|0|16|251|
|Druid / Restoration (105)|205|29|12|0|0|18|234|
|Death Knight / Blood (250)|152|31|15|0|0|16|183|
|Death Knight / Frost (251)|156|27|11|0|0|16|183|
|Death Knight / Unholy (252)|154|28|13|0|0|15|182|
|Hunter / Beast Mastery (253)|181|25|9|0|0|16|206|
|Hunter / Marksmanship (254)|184|25|9|0|0|16|209|
|Hunter / Survival (255)|184|23|8|0|0|15|207|
|Priest / Discipline (256)|146|14|8|0|0|6|160|
|Priest / Holy (257)|148|14|8|0|0|6|162|
|Priest / Shadow (258)|151|17|9|0|0|8|168|
|Rogue / Assassination (259)|183|13|3|0|0|10|196|
|Rogue / Outlaw (260)|183|14|4|0|0|10|197|
|Rogue / Subtlety (261)|187|15|4|0|0|11|202|
|Shaman / Elemental (262)|193|12|3|0|0|9|205|
|Shaman / Enhancement (263)|180|13|4|1|0|8|193|
|Shaman / Restoration (264)|188|14|4|0|0|10|202|
|Warlock / Affliction (265)|143|18|7|1|0|10|161|
|Warlock / Demonology (266)|145|20|7|1|0|12|165|
|Warlock / Destruction (267)|150|21|9|1|0|11|171|
|Monk / Brewmaster (268)|192|10|4|0|0|6|202|
|Monk / Windwalker (269)|209|12|5|0|0|7|221|
|Monk / Mistweaver (270)|203|16|10|0|0|6|219|
|Demon Hunter / Havoc (577)|161|18|11|0|0|7|179|
|Demon Hunter / Vengeance (581)|173|15|9|0|0|6|188|
|Evoker / Devastation (1467)|152|11|4|0|0|7|163|
|Evoker / Preservation (1468)|155|14|5|0|0|9|169|
|Evoker / Augmentation (1473)|144|15|6|0|0|9|159|
|Demon Hunter / Devourer (1480)|157|16|9|0|0|7|173|

Cause columns overlap. Every spell has its exact native receipt and classifications in `census.json`.

## Per-spell results

### Mage / Arcane (62)

|Spell|Name|Result|Observed causes|
|---:|---|---|---|
|66|Invisibility|PASS||
|116|Frostbolt|PASS||
|118|Polymorph|PASS||
|120|Cone of Cold|PASS||
|122|Frost Nova|PASS||
|130|Slow Fall|PASS||
|475|Remove Curse|PASS||
|1449|Arcane Explosion|PASS||
|1459|Arcane Intellect|PASS||
|1953|Blink|PASS||
|2139|Counterspell|PASS||
|3561|Teleport: Stormwind|FAIL|a:effect:77|
|3562|Teleport: Ironforge|FAIL|a:effect:77|
|3563|Teleport: Undercity|FAIL|a:effect:77|
|3565|Teleport: Darnassus|FAIL|a:effect:77|
|3566|Teleport: Thunder Bluff|FAIL|a:effect:77|
|3567|Teleport: Orgrimmar|FAIL|a:effect:77|
|5143|Arcane Missiles|FAIL|d:fixture:InvalidTarget:SPELL_FAILED_CASTER_AURASTATE|
|10059|Portal: Stormwind|PASS||
|11416|Portal: Ironforge|PASS||
|11417|Portal: Orgrimmar|PASS||
|11418|Portal: Undercity|PASS||
|11419|Portal: Darnassus|PASS||
|11420|Portal: Thunder Bluff|PASS||
|12051|Evocation|PASS||
|28271|Polymorph|PASS||
|28272|Polymorph|PASS||
|30449|Spellsteal|PASS||
|30451|Arcane Blast|PASS||
|31661|Dragon's Breath|PASS||
|32266|Portal: Exodar|PASS||
|32267|Portal: Silvermoon (Burning Crusade)|PASS||
|32271|Teleport: Exodar|FAIL|a:effect:77|
|32272|Teleport: Silvermoon (Burning Crusade)|FAIL|a:effect:77|
|33690|Teleport: Shattrath|FAIL|a:effect:77|
|33691|Portal: Shattrath|PASS||
|35715|Teleport: Shattrath|FAIL|a:effect:77|
|35717|Portal: Shattrath|PASS||
|44425|Arcane Barrage|PASS||
|45438|Ice Block|PASS||
|49358|Teleport: Stonard|FAIL|a:effect:77|
|49359|Teleport: Theramore|FAIL|a:effect:77|
|49360|Portal: Theramore|PASS||
|49361|Portal: Stonard|PASS||
|53140|Teleport: Dalaran - Northrend|FAIL|a:effect:77|
|53142|Portal: Dalaran - Northrend|PASS||
|55342|Mirror Image|PASS||
|56384|Arcane Momentum|PASS||
|61025|Polymorph|PASS||
|61305|Polymorph|PASS||
|61721|Polymorph|PASS||
|61780|Polymorph|PASS||
|79684|Clearcasting|PASS||
|80353|Time Warp|FAIL|a:aura:399|
|88342|Teleport: Tol Barad|FAIL|a:effect:77|
|88344|Teleport: Tol Barad|FAIL|a:effect:77|
|88345|Portal: Tol Barad|PASS||
|88346|Portal: Tol Barad|PASS||
|89744|Wizardry|FAIL|d:fixture:no_passive_aura|
|110959|Greater Invisibility|PASS||
|113092|missing|FAIL|d:missing_store|
|113724|Ring of Frost|PASS||
|120145|Ancient Teleport: Dalaran|FAIL|d:fixture:NotSupported:missing SpellTargetPosition row|
|120146|Ancient Portal: Dalaran|PASS||
|121039|Mana Attunement|PASS||
|125430|missing|FAIL|d:missing_store|
|126819|Polymorph|PASS||
|131784|Illusion|PASS||
|132620|Portal: Vale of Eternal Blossoms|PASS||
|132621|Teleport: Vale of Eternal Blossoms|FAIL|a:effect:77|
|132626|Portal: Vale of Eternal Blossoms|PASS||
|132627|Teleport: Vale of Eternal Blossoms|FAIL|a:effect:77|
|137018|Mage|PASS||
|137021|Arcane Mage|PASS||
|153626|Arcane Orb|PASS||
|157980|Supernova|PASS||
|157997|Ice Nova|PASS||
|161353|Polymorph|PASS||
|161354|Polymorph|PASS||
|161355|Polymorph|PASS||
|161372|Polymorph|PASS||
|162699|Stat Negation Aura - Intellect DPS|PASS||
|175689|missing|FAIL|d:missing_store|
|176242|Teleport: Warspear|FAIL|a:effect:77|
|176244|Portal: Warspear|PASS||
|176246|Portal: Stormshield|PASS||
|176248|Teleport: Stormshield|FAIL|a:effect:77|
|190336|Conjure Refreshment|PASS||
|190427|Arcane Charge|PASS||
|190740|Mastery: Savant|FAIL|a:aura:178|
|193759|Teleport: Hall of the Guardian|PASS||
|205022|Arcane Familiar|PASS||
|205025|Presence of Mind|PASS||
|205028|Resonance|PASS||
|205036|Ice Ward|PASS||
|210086|Arcane Linguist|PASS||
|210476|Master of Escape|PASS||
|212653|Shimmer|FAIL|a:aura:377|
|220647|missing|FAIL|d:missing_store|
|224869|Teleport: Dalaran - Broken Isles|FAIL|a:effect:77|
|224871|Portal: Dalaran - Broken Isles|PASS||
|226178|missing|FAIL|d:missing_store|
|231018|missing|FAIL|d:missing_store|
|231564|Arcing Cleave|PASS||
|235450|Prismatic Barrier|PASS||
|236457|Slipstream|PASS||
|236628|Amplification|PASS||
|257190|missing|FAIL|d:missing_store|
|277787|Polymorph|PASS||
|277792|Polymorph|PASS||
|281400|Portal: Boralus|PASS||
|281402|Portal: Dazar'alor|PASS||
|281403|Teleport: Boralus|FAIL|a:effect:77|
|281404|Teleport: Dazar'alor|FAIL|a:effect:77|
|281482|Reverberate|PASS||
|319836|Fire Blast|PASS||
|321387|Enlightened|PASS||
|321395|Polymorph|PASS||
|321420|Improved Clearcasting|PASS||
|321507|Touch of the Magi|FAIL|a:effect:142|
|321526|Mana Adept|PASS||
|321745|Improved Prismatic Barrier|PASS||
|342231|Arcane Echo|PASS||
|342245|Alter Time|PASS||
|342249|Master of Time|PASS||
|343127|Teleport|PASS||
|343140|Portal|PASS||
|343183|Improved Frost Nova|PASS||
|344587|Teleport: Oribos|FAIL|d:fixture:NotSupported:missing SpellTargetPosition row|
|344597|Portal: Oribos|PASS||
|365350|Arcane Surge|PASS||
|382268|Flow of Time|PASS||
|382293|Incantation of Swiftness|PASS||
|382297|Quick Witted|PASS||
|382424|Winter's Protection|PASS||
|382490|Tome of Antonidas|PASS||
|382493|Tome of Rhonin|PASS||
|383092|Arcane Warding|PASS||
|383121|Mass Polymorph|PASS||
|383676|Impetus|PASS||
|383980|Arcane Tempo|PASS||
|384060|Illuminated Thoughts|PASS||
|384452|Arcane Salvo|PASS||
|384612|Prodigious Savant|PASS||
|384651|Charged Orb|PASS||
|384858|Orb Barrage|PASS||
|386763|Freezing Cold|PASS||
|386828|Energized Barriers|PASS||
|387807|Time Manipulation|PASS||
|390218|Overflowing Energy|PASS||
|391622|Polymorph|PASS||
|395277|Teleport: Valdrakken|FAIL|d:fixture:NotSupported:missing SpellTargetPosition row|
|395289|Portal: Valdrakken|PASS||
|414379|Concentrated Power|PASS||
|414658|Ice Cold|PASS||
|414664|Mass Invisibility|PASS||
|443739|Splintering Sorcery|PASS||
|443783|Splinterstorm|PASS||
|444256|Splintering Orbs|PASS||
|444483|Controlled Instincts|PASS||
|444675|Shifting Shards|PASS||
|444719|Force of Will|PASS||
|444752|Slippery Slinging|PASS||
|444756|Look Again|PASS||
|444784|Phantasmal Image|PASS||
|444827|Reactive Barrier|PASS||
|444968|Archmage's Wrath|PASS||
|444986|Spellfrost Teachings|PASS||
|446534|Portal: Dornogal|PASS||
|446540|Teleport: Dornogal|FAIL|a:effect:77|
|448601|Spellfire Spheres|PASS||
|448658|Invocation: Arcane Phoenix|PASS||
|449293|Mana Cascade|PASS||
|449330|Merely a Setback|PASS||
|449349|Sunfury Execution|PASS||
|449382|Codex of the Sunstriders|PASS||
|449394|Glorious Incandescence|PASS||
|449412|Savor the Moment|PASS||
|449596|Rondurmancy|PASS||
|449619|Memory of Al'ar|PASS||
|449627|Lessons in Debilitation|PASS||
|451035|Burden of Power|PASS||
|452198|Eureka|PASS||
|452997|Energized Familiar|PASS||
|455428|Barrier Diffusion|PASS||
|457521|Mana Bomb|PASS||
|458437|Inspired Intellect|PASS||
|460392|Polymorph|PASS||
|461248|High Voltage|PASS||
|461251|Charged Missiles|PASS||
|461257|Focusing Crystal|PASS||
|461260|Consortium's Bauble|PASS||
|461261|Brainstorm|PASS||
|462083|Arcane Mage|PASS||
|462084|Mage|FAIL|a:aura:320|
|470021|Signature Spell|PASS||
|1223798|Intuition|PASS||
|1229376|Single-Button Assistant|PASS||
|1241462|Arcane Pulse|PASS||
|1243307|Aether Attunement|PASS||
|1243435|Orb Mastery|PASS||
|1243507|Aegwynn's Technique|PASS||
|1243542|Overflowing Insight|PASS||
|1243557|Expanded Mind|PASS||
|1244001|Arcane Singularity|PASS||
|1244025|Improved Conjuration|PASS||
|1244031|Spatial Manipulation|PASS||
|1244087|Time Walk|PASS||
|1244090|Temporal Realignment|PASS||
|1244105|Charm of Aegwynn|PASS||
|1244107|Charm of Medivh|PASS||
|1244329|Overpowered Missiles|PASS||
|1244340|Improved Blink|PASS||
|1246030|Explosive Potential|PASS||
|1250315|Frost Conditioning|PASS||
|1255166|Time Twist|PASS||
|1256909|Arcane Mage|FAIL|a:aura:646; a:aura:647; a:aura:649|
|1259190|Teleport: Silvermoon City|FAIL|d:fixture:NotSupported:missing SpellTargetPosition row|
|1259194|Portal: Silvermoon City|PASS||
|1260272|Ashes of Inspiration|PASS||
|1260616|Spellfire Salvo|PASS||
|1260673|Pyrocosm|PASS||
|1261080|Infused Splinters|PASS||
|1261082|Polished Focus|PASS||
|1261106|Attuned Familiar|PASS||
|1265517|Permafrost Bauble|PASS||
|1270827|Improved Spellsteal|PASS||
|1270829|Reflection|PASS||
|1270845|Mana Confluence|FAIL|a:aura:423|
|1270847|Improved Remove Curse|PASS||
|1270865|Improved Counterspell|PASS||
|1270872|Captured Thoughts|PASS||
|1280165|Augury Abounds|PASS||
|1297073|Improved Warding|PASS||
|1309497|Refractive Images|PASS||

### Mage / Fire (63)

|Spell|Name|Result|Observed causes|
|---:|---|---|---|
|66|Invisibility|PASS||
|116|Frostbolt|PASS||
|118|Polymorph|PASS||
|120|Cone of Cold|PASS||
|122|Frost Nova|PASS||
|130|Slow Fall|PASS||
|133|Fireball|PASS||
|475|Remove Curse|PASS||
|1449|Arcane Explosion|PASS||
|1459|Arcane Intellect|PASS||
|1953|Blink|PASS||
|2120|Flamestrike|PASS||
|2139|Counterspell|PASS||
|2948|Scorch|PASS||
|3561|Teleport: Stormwind|FAIL|a:effect:77|
|3562|Teleport: Ironforge|FAIL|a:effect:77|
|3563|Teleport: Undercity|FAIL|a:effect:77|
|3565|Teleport: Darnassus|FAIL|a:effect:77|
|3566|Teleport: Thunder Bluff|FAIL|a:effect:77|
|3567|Teleport: Orgrimmar|FAIL|a:effect:77|
|10059|Portal: Stormwind|PASS||
|11366|Pyroblast|PASS||
|11416|Portal: Ironforge|PASS||
|11417|Portal: Orgrimmar|PASS||
|11418|Portal: Undercity|PASS||
|11419|Portal: Darnassus|PASS||
|11420|Portal: Thunder Bluff|PASS||
|12846|Mastery: Ignite|PASS||
|28271|Polymorph|PASS||
|28272|Polymorph|PASS||
|30449|Spellsteal|PASS||
|30482|missing|FAIL|d:missing_store|
|31661|Dragon's Breath|PASS||
|32266|Portal: Exodar|PASS||
|32267|Portal: Silvermoon (Burning Crusade)|PASS||
|32271|Teleport: Exodar|FAIL|a:effect:77|
|32272|Teleport: Silvermoon (Burning Crusade)|FAIL|a:effect:77|
|33690|Teleport: Shattrath|FAIL|a:effect:77|
|33691|Portal: Shattrath|PASS||
|35715|Teleport: Shattrath|FAIL|a:effect:77|
|35717|Portal: Shattrath|PASS||
|44448|Pyroblast Clearcasting Driver|PASS||
|45438|Ice Block|PASS||
|49358|Teleport: Stonard|FAIL|a:effect:77|
|49359|Teleport: Theramore|FAIL|a:effect:77|
|49360|Portal: Theramore|PASS||
|49361|Portal: Stonard|PASS||
|53140|Teleport: Dalaran - Northrend|FAIL|a:effect:77|
|53142|Portal: Dalaran - Northrend|PASS||
|55342|Mirror Image|PASS||
|56384|Arcane Momentum|PASS||
|61025|Polymorph|PASS||
|61305|Polymorph|PASS||
|61721|Polymorph|PASS||
|61780|Polymorph|PASS||
|80353|Time Warp|FAIL|a:aura:399|
|86949|Cauterize|PASS||
|88342|Teleport: Tol Barad|FAIL|a:effect:77|
|88344|Teleport: Tol Barad|FAIL|a:effect:77|
|88345|Portal: Tol Barad|PASS||
|88346|Portal: Tol Barad|PASS||
|89744|Wizardry|FAIL|d:fixture:no_passive_aura|
|108853|Fire Blast|PASS||
|110959|Greater Invisibility|PASS||
|113092|missing|FAIL|d:missing_store|
|113724|Ring of Frost|PASS||
|117216|Critical Mass|PASS||
|120145|Ancient Teleport: Dalaran|FAIL|d:fixture:NotSupported:missing SpellTargetPosition row|
|120146|Ancient Portal: Dalaran|PASS||
|121039|Mana Attunement|PASS||
|125430|missing|FAIL|d:missing_store|
|126819|Polymorph|PASS||
|131784|Illusion|PASS||
|132620|Portal: Vale of Eternal Blossoms|PASS||
|132621|Teleport: Vale of Eternal Blossoms|FAIL|a:effect:77|
|132626|Portal: Vale of Eternal Blossoms|PASS||
|132627|Teleport: Vale of Eternal Blossoms|FAIL|a:effect:77|
|137018|Mage|PASS||
|137019|Fire Mage|PASS||
|153561|Meteor|PASS||
|157980|Supernova|PASS||
|157997|Ice Nova|PASS||
|161353|Polymorph|PASS||
|161354|Polymorph|PASS||
|161355|Polymorph|PASS||
|161372|Polymorph|PASS||
|162699|Stat Negation Aura - Intellect DPS|PASS||
|175689|missing|FAIL|d:missing_store|
|176242|Teleport: Warspear|FAIL|a:effect:77|
|176244|Portal: Warspear|PASS||
|176246|Portal: Stormshield|PASS||
|176248|Teleport: Stormshield|FAIL|a:effect:77|
|190319|Combustion|PASS||
|190336|Conjure Refreshment|PASS||
|193759|Teleport: Hall of the Guardian|PASS||
|195283|Hot Streak|FAIL|d:fixture:no_passive_aura|
|205026|Firestarter|PASS||
|205029|Flame On|PASS||
|205036|Ice Ward|PASS||
|210086|Arcane Linguist|PASS||
|210476|Master of Escape|PASS||
|212653|Shimmer|FAIL|a:aura:377|
|220647|missing|FAIL|d:missing_store|
|224869|Teleport: Dalaran - Broken Isles|FAIL|a:effect:77|
|224871|Portal: Dalaran - Broken Isles|PASS||
|226178|missing|FAIL|d:missing_store|
|231018|missing|FAIL|d:missing_store|
|235313|Blazing Barrier|PASS||
|257190|missing|FAIL|d:missing_store|
|269650|Pyroclasm|PASS||
|277787|Polymorph|PASS||
|277792|Polymorph|PASS||
|281400|Portal: Boralus|PASS||
|281402|Portal: Dazar'alor|PASS||
|281403|Teleport: Boralus|FAIL|a:effect:77|
|281404|Teleport: Dazar'alor|FAIL|a:effect:77|
|319836|Fire Blast|PASS||
|321395|Polymorph|PASS||
|321708|Improved Blazing Barrier|PASS||
|342245|Alter Time|PASS||
|342249|Master of Time|PASS||
|342344|From the Ashes|PASS||
|343127|Teleport|PASS||
|343140|Portal|PASS||
|343183|Improved Frost Nova|PASS||
|343230|Mote of Flame|PASS||
|344587|Teleport: Oribos|FAIL|d:fixture:NotSupported:missing SpellTargetPosition row|
|344597|Portal: Oribos|PASS||
|382268|Flow of Time|PASS||
|382293|Incantation of Swiftness|PASS||
|382297|Quick Witted|PASS||
|382424|Winter's Protection|PASS||
|382490|Tome of Antonidas|PASS||
|382493|Tome of Rhonin|PASS||
|383092|Arcane Warding|PASS||
|383121|Mass Polymorph|PASS||
|383391|Feel the Burn|PASS||
|383634|Fiery Rush|PASS||
|383669|Controlled Destruction|PASS||
|383810|Fevered Incantation|PASS||
|383967|Slow Burn|PASS||
|384174|Master of Flame|PASS||
|386763|Freezing Cold|PASS||
|386828|Energized Barriers|PASS||
|387044|Fervent Flickering|PASS||
|387807|Time Manipulation|PASS||
|390218|Overflowing Energy|PASS||
|391622|Polymorph|PASS||
|395277|Teleport: Valdrakken|FAIL|d:fixture:NotSupported:missing SpellTargetPosition row|
|395289|Portal: Valdrakken|PASS||
|414658|Ice Cold|PASS||
|414664|Mass Invisibility|PASS||
|416094|Fuel the Fire|PASS||
|416714|Intensifying Flame|PASS||
|416719|Blast Zone|PASS||
|417467|Inflame|PASS||
|431044|Frostfire Bolt|PASS||
|431066|Imbued Warding|PASS||
|431067|Elemental Affinity|PASS||
|431095|Isothermic Core|PASS||
|431112|Flame and Frost|PASS||
|431117|Thermal Conditioning|PASS||
|431131|Meltdown|PASS||
|431166|Frostfire Infusion|PASS||
|431176|Frostfire Empowerment|PASS||
|431178|Flash Freezeburn|PASS||
|431189|Severe Temperatures|PASS||
|446534|Portal: Dornogal|PASS||
|446540|Teleport: Dornogal|FAIL|a:effect:77|
|448601|Spellfire Spheres|PASS||
|448658|Invocation: Arcane Phoenix|PASS||
|449293|Mana Cascade|PASS||
|449330|Merely a Setback|PASS||
|449349|Sunfury Execution|PASS||
|449382|Codex of the Sunstriders|PASS||
|449394|Glorious Incandescence|PASS||
|449412|Savor the Moment|PASS||
|449596|Rondurmancy|PASS||
|449619|Memory of Al'ar|PASS||
|449627|Lessons in Debilitation|PASS||
|450746|Scald|PASS||
|450831|Fire's Ire|PASS||
|451035|Burden of Power|PASS||
|451466|Pyromaniac|PASS||
|451875|Spontaneous Combustion|PASS||
|453282|Flame Accelerant|PASS||
|455428|Barrier Diffusion|PASS||
|457735|Heat Shimmer|PASS||
|457803|Molten Fury|FAIL|a:aura:303|
|458437|Inspired Intellect|PASS||
|460392|Polymorph|PASS||
|461261|Brainstorm|PASS||
|462084|Mage|FAIL|a:aura:320|
|462085|Fire Mage|PASS||
|1229376|Single-Button Assistant|PASS||
|1244025|Improved Conjuration|PASS||
|1244031|Spatial Manipulation|PASS||
|1244087|Time Walk|PASS||
|1244090|Temporal Realignment|PASS||
|1244105|Charm of Aegwynn|PASS||
|1244107|Charm of Medivh|PASS||
|1244340|Improved Blink|PASS||
|1246030|Explosive Potential|PASS||
|1248002|Heat Sink|FAIL|a:aura:220|
|1248014|Dualcasting Adept|PASS||
|1248016|Blast Radius|PASS||
|1250315|Frost Conditioning|PASS||
|1254194|Kindling|PASS||
|1254512|Burn It All|PASS||
|1254756|Ignition|PASS||
|1254828|Wildfire|PASS||
|1254851|Flamestrike|PASS||
|1254857|Deep Impact|PASS||
|1254865|Cinderstorm|PASS||
|1255166|Time Twist|PASS||
|1256911|Fire Mage|FAIL|a:aura:646; a:aura:647; a:aura:648; a:aura:649|
|1259190|Teleport: Silvermoon City|FAIL|d:fixture:NotSupported:missing SpellTargetPosition row|
|1259194|Portal: Silvermoon City|PASS||
|1260272|Ashes of Inspiration|PASS||
|1260616|Spellfire Salvo|PASS||
|1260673|Pyrocosm|PASS||
|1262843|Duality|PASS||
|1262844|Molten Chill|PASS||
|1262845|Elemental Conduit|PASS||
|1265517|Permafrost Bauble|PASS||
|1270827|Improved Spellsteal|PASS||
|1270829|Reflection|PASS||
|1270845|Mana Confluence|FAIL|a:aura:423|
|1270847|Improved Remove Curse|PASS||
|1270865|Improved Counterspell|PASS||
|1270872|Captured Thoughts|PASS||
|1271171|Conflagration|PASS||
|1271177|Burnout|PASS||
|1297073|Improved Warding|PASS||

### Mage / Frost (64)

|Spell|Name|Result|Observed causes|
|---:|---|---|---|
|66|Invisibility|PASS||
|116|Frostbolt|PASS||
|118|Polymorph|PASS||
|120|Cone of Cold|PASS||
|122|Frost Nova|PASS||
|130|Slow Fall|PASS||
|475|Remove Curse|PASS||
|1449|Arcane Explosion|PASS||
|1459|Arcane Intellect|PASS||
|1953|Blink|PASS||
|2139|Counterspell|PASS||
|3561|Teleport: Stormwind|FAIL|a:effect:77|
|3562|Teleport: Ironforge|FAIL|a:effect:77|
|3563|Teleport: Undercity|FAIL|a:effect:77|
|3565|Teleport: Darnassus|FAIL|a:effect:77|
|3566|Teleport: Thunder Bluff|FAIL|a:effect:77|
|3567|Teleport: Orgrimmar|FAIL|a:effect:77|
|10059|Portal: Stormwind|PASS||
|11416|Portal: Ironforge|PASS||
|11417|Portal: Orgrimmar|PASS||
|11418|Portal: Undercity|PASS||
|11419|Portal: Darnassus|PASS||
|11420|Portal: Thunder Bluff|PASS||
|11426|Ice Barrier|PASS||
|28271|Polymorph|PASS||
|28272|Polymorph|PASS||
|30449|Spellsteal|PASS||
|30455|Ice Lance|PASS||
|31661|Dragon's Breath|PASS||
|31687|Summon Water Elemental|PASS||
|32266|Portal: Exodar|PASS||
|32267|Portal: Silvermoon (Burning Crusade)|PASS||
|32271|Teleport: Exodar|FAIL|a:effect:77|
|32272|Teleport: Silvermoon (Burning Crusade)|FAIL|a:effect:77|
|33690|Teleport: Shattrath|FAIL|a:effect:77|
|33691|Portal: Shattrath|PASS||
|35715|Teleport: Shattrath|FAIL|a:effect:77|
|35717|Portal: Shattrath|PASS||
|44614|Flurry|PASS||
|45438|Ice Block|PASS||
|49358|Teleport: Stonard|FAIL|a:effect:77|
|49359|Teleport: Theramore|FAIL|a:effect:77|
|49360|Portal: Theramore|PASS||
|49361|Portal: Stonard|PASS||
|53140|Teleport: Dalaran - Northrend|FAIL|a:effect:77|
|53142|Portal: Dalaran - Northrend|PASS||
|55342|Mirror Image|PASS||
|56377|Splitting Ice|PASS||
|56384|Arcane Momentum|PASS||
|61025|Polymorph|PASS||
|61305|Polymorph|PASS||
|61721|Polymorph|PASS||
|61780|Polymorph|PASS||
|80353|Time Warp|FAIL|a:aura:399|
|84714|Frozen Orb|PASS||
|88342|Teleport: Tol Barad|FAIL|a:effect:77|
|88344|Teleport: Tol Barad|FAIL|a:effect:77|
|88345|Portal: Tol Barad|PASS||
|88346|Portal: Tol Barad|PASS||
|89744|Wizardry|FAIL|d:fixture:no_passive_aura|
|110959|Greater Invisibility|PASS||
|112965|Fingers of Frost|PASS||
|113092|missing|FAIL|d:missing_store|
|113724|Ring of Frost|PASS||
|120145|Ancient Teleport: Dalaran|FAIL|d:fixture:NotSupported:missing SpellTargetPosition row|
|120146|Ancient Portal: Dalaran|PASS||
|121039|Mana Attunement|PASS||
|125430|missing|FAIL|d:missing_store|
|126819|Polymorph|PASS||
|131784|Illusion|PASS||
|132620|Portal: Vale of Eternal Blossoms|PASS||
|132621|Teleport: Vale of Eternal Blossoms|FAIL|a:effect:77|
|132626|Portal: Vale of Eternal Blossoms|PASS||
|132627|Teleport: Vale of Eternal Blossoms|FAIL|a:effect:77|
|137018|Mage|PASS||
|137020|Frost Mage|PASS||
|157980|Supernova|PASS||
|157997|Ice Nova|PASS||
|161353|Polymorph|PASS||
|161354|Polymorph|PASS||
|161355|Polymorph|PASS||
|161372|Polymorph|PASS||
|162699|Stat Negation Aura - Intellect DPS|PASS||
|175689|missing|FAIL|d:missing_store|
|176242|Teleport: Warspear|FAIL|a:effect:77|
|176244|Portal: Warspear|PASS||
|176246|Portal: Stormshield|PASS||
|176248|Teleport: Stormshield|FAIL|a:effect:77|
|190336|Conjure Refreshment|PASS||
|190356|Blizzard|PASS||
|190447|Brain Freeze|PASS||
|193759|Teleport: Hall of the Guardian|PASS||
|205021|Ray of Frost|PASS||
|205024|Lonely Winter|PASS||
|205030|Frozen Touch|PASS||
|205036|Ice Ward|PASS||
|210086|Arcane Linguist|PASS||
|210476|Master of Escape|PASS||
|212653|Shimmer|FAIL|a:aura:377|
|220647|missing|FAIL|d:missing_store|
|224869|Teleport: Dalaran - Broken Isles|FAIL|a:effect:77|
|224871|Portal: Dalaran - Broken Isles|PASS||
|226178|missing|FAIL|d:missing_store|
|231018|missing|FAIL|d:missing_store|
|235219|Cold Snap|PASS||
|270233|Freezing Rain|PASS||
|277787|Polymorph|PASS||
|277792|Polymorph|PASS||
|281400|Portal: Boralus|PASS||
|281402|Portal: Dazar'alor|PASS||
|281403|Teleport: Boralus|FAIL|a:effect:77|
|281404|Teleport: Dazar'alor|FAIL|a:effect:77|
|319836|Fire Blast|PASS||
|321395|Polymorph|PASS||
|342245|Alter Time|PASS||
|342249|Master of Time|PASS||
|343127|Teleport|PASS||
|343140|Portal|PASS||
|343183|Improved Frost Nova|PASS||
|344587|Teleport: Oribos|FAIL|d:fixture:NotSupported:missing SpellTargetPosition row|
|344597|Portal: Oribos|PASS||
|378406|Wintertide|PASS||
|378448|Fractured Frost|PASS||
|378919|Piercing Cold|PASS||
|378947|Glacial Assault|PASS||
|379993|Flash Freeze|PASS||
|382268|Flow of Time|PASS||
|382293|Incantation of Swiftness|PASS||
|382297|Quick Witted|PASS||
|382424|Winter's Protection|PASS||
|382490|Tome of Antonidas|PASS||
|382493|Tome of Rhonin|PASS||
|383092|Arcane Warding|PASS||
|383121|Mass Polymorph|PASS||
|385167|Everlasting Frost|PASS||
|386763|Freezing Cold|PASS||
|386828|Energized Barriers|PASS||
|387807|Time Manipulation|PASS||
|390218|Overflowing Energy|PASS||
|391622|Polymorph|PASS||
|395277|Teleport: Valdrakken|FAIL|d:fixture:NotSupported:missing SpellTargetPosition row|
|395289|Portal: Valdrakken|PASS||
|414658|Ice Cold|PASS||
|414664|Mass Invisibility|PASS||
|417489|Winter's Blessing|PASS||
|418733|Splintering Ray|PASS||
|431044|Frostfire Bolt|PASS||
|431066|Imbued Warding|PASS||
|431067|Elemental Affinity|PASS||
|431095|Isothermic Core|PASS||
|431112|Flame and Frost|PASS||
|431117|Thermal Conditioning|PASS||
|431131|Meltdown|PASS||
|431166|Frostfire Infusion|PASS||
|431176|Frostfire Empowerment|PASS||
|431178|Flash Freezeburn|PASS||
|431189|Severe Temperatures|PASS||
|443739|Splintering Sorcery|PASS||
|443783|Splinterstorm|PASS||
|444256|Splintering Orbs|PASS||
|444483|Controlled Instincts|PASS||
|444675|Shifting Shards|PASS||
|444719|Force of Will|PASS||
|444752|Slippery Slinging|PASS||
|444756|Look Again|PASS||
|444784|Phantasmal Image|PASS||
|444827|Reactive Barrier|PASS||
|444968|Archmage's Wrath|PASS||
|444986|Spellfrost Teachings|PASS||
|446534|Portal: Dornogal|PASS||
|446540|Teleport: Dornogal|FAIL|a:effect:77|
|455428|Barrier Diffusion|PASS||
|458437|Inspired Intellect|PASS||
|460392|Polymorph|PASS||
|460590|Permafrost Lances|PASS||
|461261|Brainstorm|PASS||
|462084|Mage|FAIL|a:aura:320|
|462086|Frost Mage|PASS||
|470021|Signature Spell|PASS||
|1216953|Freezing Winds|PASS||
|1229376|Single-Button Assistant|PASS||
|1244025|Improved Conjuration|PASS||
|1244031|Spatial Manipulation|PASS||
|1244069|Improved Ice Barrier|PASS||
|1244087|Time Walk|PASS||
|1244090|Temporal Realignment|PASS||
|1244105|Charm of Aegwynn|PASS||
|1244107|Charm of Medivh|PASS||
|1244110|Glacial Bulwark|PASS||
|1244340|Improved Blink|PASS||
|1246752|Mastery: Freeze and Shatter|PASS||
|1246769|Shatter|PASS||
|1246811|Improved Shatter|PASS||
|1246832|Icicles|PASS||
|1247025|Deep Shatter|PASS||
|1247055|Frigid Focus|PASS||
|1247090|Cone of Frost|PASS||
|1247544|Crystalline Refraction|PASS||
|1247729|Thermal Void|PASS||
|1247742|Hailstones|PASS||
|1247743|Rimecaster|PASS||
|1247759|Glacial Attunement|PASS||
|1247769|Improved Flurry|PASS||
|1247775|Winter's End|PASS||
|1247777|Comet Storm|PASS||
|1247798|Glaciate|PASS||
|1247799|Heart of Ice|PASS||
|1248002|Heat Sink|FAIL|a:aura:220|
|1248014|Dualcasting Adept|PASS||
|1248016|Blast Radius|PASS||
|1248825|Frostbite|PASS||
|1248829|Blizzard|PASS||
|1250315|Frost Conditioning|PASS||
|1256913|Frost Mage|FAIL|a:aura:646; a:aura:647; a:aura:649|
|1259190|Teleport: Silvermoon City|FAIL|d:fixture:NotSupported:missing SpellTargetPosition row|
|1259194|Portal: Silvermoon City|PASS||
|1261080|Infused Splinters|PASS||
|1261082|Polished Focus|PASS||
|1261106|Attuned Familiar|PASS||
|1262843|Duality|PASS||
|1262844|Molten Chill|PASS||
|1262845|Elemental Conduit|PASS||
|1265517|Permafrost Bauble|PASS||
|1270827|Improved Spellsteal|PASS||
|1270829|Reflection|PASS||
|1270845|Mana Confluence|FAIL|a:aura:423|
|1270847|Improved Remove Curse|PASS||
|1270865|Improved Counterspell|PASS||
|1270872|Captured Thoughts|PASS||
|1280165|Augury Abounds|PASS||
|1280779|Glacial Chill|PASS||
|1280780|Glacial Shatter|PASS||
|1281638|White Out|PASS||
|1297073|Improved Warding|PASS||

### Paladin / Holy (65)

|Spell|Name|Result|Observed causes|
|---:|---|---|---|
|498|Divine Protection|PASS||
|633|Lay on Hands|FAIL|a:effect:67|
|642|Divine Shield|PASS||
|853|Hammer of Justice|PASS||
|1022|Blessing of Protection|PASS||
|1044|Blessing of Freedom|PASS||
|4987|Cleanse|PASS||
|5502|Sense Undead|PASS||
|6940|Blessing of Sacrifice|FAIL|d:fixture:self_with_exclude_caster|
|7328|Redemption|PASS||
|10326|Turn Evil|FAIL|d:fixture:ineligible_creature_type|
|13819|Summon Warhorse|FAIL|a:effect:77|
|19750|Flash of Light|PASS||
|20271|Judgment|PASS||
|20473|Holy Shock|PASS||
|23214|Summon Charger|FAIL|a:effect:77|
|26573|Consecration|PASS||
|31821|Aura Mastery|PASS||
|31884|Avenging Wrath|PASS||
|34767|Summon Thalassian Charger|FAIL|a:effect:77|
|34769|Summon Thalassian Warhorse|FAIL|a:effect:77|
|35395|Crusader Strike|PASS||
|53376|Sanctified Wrath|PASS||
|53563|Beacon of Light|PASS||
|53576|Infusion of Light|PASS||
|53600|Shield of the Righteous|PASS||
|53651|Light's Beacon|PASS||
|62124|Hand of Reckoning|PASS||
|66235|Ardent Defender|PASS||
|69820|Summon Sunwalker Kodo|FAIL|a:effect:77|
|69826|Summon Great Sunwalker Kodo|FAIL|a:effect:77|
|73629|Summon Exarch's Elekk|FAIL|a:effect:77|
|73630|Summon Great Exarch's Elekk|FAIL|a:effect:77|
|81297|Consecration|PASS||
|82242|Parry|PASS||
|82326|Holy Light|PASS||
|85222|Light of Dawn|PASS||
|85673|Word of Glory|PASS||
|86103|Plate Specialization|FAIL|d:fixture:no_passive_aura|
|86678|Light of the Ancient Kings|PASS||
|86704|Ancient Fury|PASS||
|94289|Protector of the Innocent|PASS||
|114154|Unbreakable Spirit|PASS||
|114165|Holy Prism|PASS||
|115675|Boundless Conviction|PASS||
|115750|Blinding Light|PASS||
|121183|Contemplation|PASS||
|123830|Block|PASS||
|137026|Paladin|PASS||
|137029|Holy Paladin|PASS||
|146956|Light's Revocation|PASS||
|156322|Eternal Flame|PASS||
|156910|Beacon of Faith|PASS||
|157047|Saved by the Light|PASS||
|162056|Jailer's Judgment|PASS||
|162701|Stat Negation Aura - Intellect Healer|PASS||
|183416|Blessing of Dawn|PASS||
|183997|Mastery: Lightbringer|PASS||
|190784|Divine Steed|PASS||
|196926|Crusader's Might|PASS||
|197446|Beacon of the Lightbringer|PASS||
|199422|Holy Ritual|PASS||
|200025|Beacon of Virtue|PASS||
|200430|Protection of Tyr|PASS||
|204054|Consecrated Ground|PASS||
|212056|Absolution|PASS||
|223817|Divine Purpose|PASS||
|226178|missing|FAIL|d:missing_store|
|230332|Cavalier|PASS||
|231018|missing|FAIL|d:missing_store|
|231642|Tower of Radiance|PASS||
|231644|Greater Judgment|PASS||
|234299|Fist of Justice|PASS||
|270562|Summon Darkforge Ram|FAIL|a:effect:77|
|270564|Summon Dawnforge Ram|FAIL|a:effect:77|
|275773|Judgment|PASS||
|289941|Holy Shock|PASS||
|290608|Crusader's Direhorn|FAIL|a:effect:77|
|305394|Unbound Freedom|PASS||
|315867|Judgment|PASS||
|327977|Judgment|PASS||
|363613|Summon Lightforged Ruinstrider|FAIL|a:effect:77|
|375576|Divine Toll|PASS||
|376996|Seasoned Warhorse|PASS||
|377128|Golden Path|PASS||
|379391|Quickened Invocation|PASS||
|384820|Sacrifice of the Just|PASS||
|384909|Improved Blessing of Protection|PASS||
|384914|Recompense|PASS||
|385414|Afterimage|PASS||
|385427|Obduracy|PASS||
|385450|Seal of Might|PASS||
|385515|Holy Aegis|PASS||
|385633|Auras of the Resolute|PASS||
|386738|Divine Resonance|PASS||
|387781|Commanding Light|PASS||
|387786|Moment of Compassion|PASS||
|387791|Empyreal Ward|PASS||
|387801|Echoing Blessings|PASS||
|387805|Divine Glimpse|PASS||
|387808|Divine Revelations|PASS||
|387879|Breaking Dawn|PASS||
|391054|Intercession|PASS||
|392902|Resplendent Light|PASS||
|392907|Inflorescence of the Sunwell|PASS||
|392911|Unwavering Spirit|PASS||
|392938|Veneration|PASS||
|392961|Imbued Infusions|PASS||
|393024|Improved Cleanse|PASS||
|394088|Avenging Crusader|PASS||
|402964|Sanctified Plates|PASS||
|404357|Guided Prayer|PASS||
|406101|Faith's Armor|PASS||
|406468|Lightforged Blessing|PASS||
|414073|Light's Conviction|PASS||
|414113|Righteous Judgment|PASS||
|414195|Awakening|PASS||
|414443|Shining Righteousness|PASS||
|414720|Tirion's Devotion|PASS||
|415091|Shield of the Righteous|PASS||
|415364|Reclamation|PASS||
|417855|Awestruck|PASS||
|425518|Light's Deliverance|PASS||
|427445|Light's Guidance|PASS||
|428076|Holy Paladin|FAIL|a:aura:647; a:aura:648; a:aura:649|
|431377|Dawnlight|PASS||
|431402|Luminosity|PASS||
|431404|Solar Grace|PASS||
|431406|Will of the Dawn|PASS||
|431407|Lingering Radiance|PASS||
|431413|Sun Sear|PASS||
|431423|Illumine|PASS||
|431425|Sun's Avatar|PASS||
|431463|Zealous Vindication|PASS||
|431474|Second Sunrise|PASS||
|431480|Gleaming Rays|PASS||
|431482|Morning Star|PASS||
|431533|Shake the Heavens|PASS||
|431551|Wrathful Descent|PASS||
|431687|Higher Calling|PASS||
|431730|Sacrosanct Crusade|PASS||
|432463|Hammerfall|PASS||
|432615|Endless Wrath|PASS||
|432626|Undisputed Ruling|PASS||
|432802|Solidarity|PASS||
|432804|Forewarning|PASS||
|432821|Shared Resolve|PASS||
|432866|Laying Down Arms|PASS||
|432919|Valiance|PASS||
|432929|Divine Hammer|PASS||
|432964|Divine Inspiration|PASS||
|432977|Sanctification|PASS||
|432990|Unrelenting Charger|PASS||
|432992|Bonds of Fellowship|PASS||
|433011|Blessing of the Forge|PASS||
|433015|Blessed Assurance|PASS||
|433106|Divine Guidance|PASS||
|433568|Rite of Sanctification|PASS||
|433583|Rite of Adjuration|PASS||
|433718|Hammer and Anvil|PASS||
|439760|Aurora|PASS||
|440013|Cleanse Toxins|PASS||
|445200|Blessing of An'she|PASS||
|447985|Light of the Martyr|PASS||
|448040|Bestow Light|PASS||
|453785|Earthen Ordinant's Ramolith|FAIL|a:effect:77|
|460720|Wrench Evil|PASS||
|461243|Light's Protection|PASS||
|461244|Overflowing Light|PASS||
|461246|Glorious Dawn|PASS||
|461273|Truth Prevails|PASS||
|461278|Extrication|PASS||
|461287|Liberation|PASS||
|462092|Paladin|FAIL|a:aura:320|
|462093|Holy Paladin|PASS||
|469279|Worthy Sacrifice|PASS||
|469304|Steed of Liberty|PASS||
|469309|Eye for an Eye|PASS||
|469316|Stoicism|PASS||
|469317|Stand Against Evil|PASS||
|469321|Righteous Protection|PASS||
|469325|Light's Countenance|PASS||
|469337|Sacred Strength|PASS||
|469409|Divine Spurs|PASS||
|469411|A Just Reward|PASS||
|469416|Lightbearer|PASS||
|469434|Selfless Healer|PASS||
|469439|Inspired Guard|PASS||
|469445|Holy Reprieve|PASS||
|469476|Divine Reach|PASS||
|469701|Tempered in Battle|PASS||
|469770|Blessed Calling|PASS||
|469780|Lead the Charge|PASS||
|469886|Authoritative Rebuke|PASS||
|1229376|Single-Button Assistant|PASS||
|1232616|Pillars of Light|PASS||
|1241275|Tyr's Deliverance|PASS||
|1241288|Hammer of Wrath|PASS||
|1241358|Empyrean Legacy|PASS||
|1241511|Call of the Righteous|PASS||
|1241542|Ringing of the Heavens|PASS||
|1241714|Seraphic Barrier|PASS||
|1241805|Glistening Radiance|PASS||
|1241945|Blessing of Dusk|PASS||
|1241958|Vengeful Wrath|PASS||
|1242008|Hand of Divinity|PASS||
|1258016|Holy Paladin|PASS||
|1260429|Divine Exaction|PASS||
|1261525|Light's Judicator|PASS||
|1263252|Seal of the Templar|PASS||
|1263782|Walk Into Light|PASS||
|1263787|Endless Gleam|PASS||
|1263920|Born in Sunlight|PASS||
|1265541|Fear No Evil|PASS||
|1265549|Brought to Light|PASS||
|1270916|Divine Favor|PASS||
|1271016|Seek Deliverance|PASS||
|1271077|Divine Overload|PASS||
|1271221|Unending Light|PASS||
|1271387|Masterwork|PASS||
|1271466|Reflection of Radiance|PASS||
|1271553|Resounding Strike|PASS||
|1277443|Armory of Light|PASS||
|1277651|Rising Sunlight|PASS||
|1289728|Holy Armaments|PASS||

### Paladin / Protection (66)

|Spell|Name|Result|Observed causes|
|---:|---|---|---|
|633|Lay on Hands|FAIL|a:effect:67|
|642|Divine Shield|PASS||
|853|Hammer of Justice|PASS||
|1022|Blessing of Protection|PASS||
|1044|Blessing of Freedom|PASS||
|5502|Sense Undead|PASS||
|6940|Blessing of Sacrifice|FAIL|d:fixture:self_with_exclude_caster|
|7328|Redemption|PASS||
|10326|Turn Evil|FAIL|d:fixture:ineligible_creature_type|
|13819|Summon Warhorse|FAIL|a:effect:77|
|19750|Flash of Light|PASS||
|20271|Judgment|PASS||
|23214|Summon Charger|FAIL|a:effect:77|
|25780|Righteous Fury|PASS||
|26573|Consecration|PASS||
|31850|Ardent Defender|PASS||
|31884|Avenging Wrath|PASS||
|31935|Avenger's Shield|PASS||
|34767|Summon Thalassian Charger|FAIL|a:effect:77|
|34769|Summon Thalassian Warhorse|FAIL|a:effect:77|
|35395|Crusader Strike|PASS||
|53595|Hammer of the Righteous|PASS||
|53600|Shield of the Righteous|PASS||
|62124|Hand of Reckoning|PASS||
|66235|Ardent Defender|PASS||
|69820|Summon Sunwalker Kodo|FAIL|a:effect:77|
|69826|Summon Great Sunwalker Kodo|FAIL|a:effect:77|
|73629|Summon Exarch's Elekk|FAIL|a:effect:77|
|73630|Summon Great Exarch's Elekk|FAIL|a:effect:77|
|76671|Mastery: Divine Bulwark|PASS||
|81297|Consecration|PASS||
|82242|Parry|PASS||
|85043|Grand Crusader|PASS||
|85673|Word of Glory|PASS||
|86102|Plate Specialization|FAIL|d:fixture:no_passive_aura|
|86659|Guardian of Ancient Kings|PASS||
|86678|Light of the Ancient Kings|PASS||
|86704|Ancient Fury|PASS||
|94289|Protector of the Innocent|PASS||
|96231|Rebuke|PASS||
|105805|Sanctuary|PASS||
|114154|Unbreakable Spirit|PASS||
|115675|Boundless Conviction|PASS||
|115750|Blinding Light|PASS||
|121183|Contemplation|PASS||
|123830|Block|PASS||
|137026|Paladin|PASS||
|137028|Protection Paladin|PASS||
|146956|Light's Revocation|PASS||
|156322|Eternal Flame|PASS||
|161800|Riposte|PASS||
|162056|Jailer's Judgment|PASS||
|162702|Stat Negation Aura - Strength Tank|PASS||
|183416|Blessing of Dawn|PASS||
|190784|Divine Steed|PASS||
|199422|Holy Ritual|PASS||
|204018|Blessing of Spellwarding|PASS||
|204019|Blessed Hammer|PASS||
|204023|Crusader's Judgment|PASS||
|204054|Consecrated Ground|PASS||
|204074|Righteous Protector|PASS||
|204077|Final Stand|PASS||
|209389|Bulwark of Order|PASS||
|213644|Cleanse Toxins|PASS||
|223817|Divine Purpose|PASS||
|226178|missing|FAIL|d:missing_store|
|230332|Cavalier|PASS||
|231018|missing|FAIL|d:missing_store|
|231663|Greater Judgment|PASS||
|234299|Fist of Justice|PASS||
|270562|Summon Darkforge Ram|FAIL|a:effect:77|
|270564|Summon Dawnforge Ram|FAIL|a:effect:77|
|275779|Judgment|PASS||
|280373|Redoubt|PASS||
|290608|Crusader's Direhorn|FAIL|a:effect:77|
|305394|Unbound Freedom|PASS||
|315867|Judgment|PASS||
|315921|Word of Glory|PASS||
|315924|Hand of the Protector|PASS||
|317907|Mastery: Divine Bulwark|FAIL|a:aura:529|
|321136|Shining Light|PASS||
|327977|Judgment|PASS||
|327980|Consecration|PASS||
|337287|missing|FAIL|d:missing_store|
|342348|Crusader Strike|PASS||
|344172|missing|FAIL|d:missing_store|
|353367|Aegis of Light|PASS||
|358934|Aegis of Light|PASS||
|363613|Summon Lightforged Ruinstrider|FAIL|a:effect:77|
|375576|Divine Toll|PASS||
|376996|Seasoned Warhorse|PASS||
|377053|Seal of Reprisal|PASS||
|377128|Golden Path|PASS||
|378279|Gift of the Golden Val'kyr|PASS||
|378285|Tyr's Enforcer|PASS||
|378405|Light of the Titans|PASS||
|378425|Uther's Counsel|PASS||
|378457|Soaring Shield|PASS||
|378762|Ferren Marcus's Fervor|PASS||
|378845|Focused Enmity|PASS||
|379021|Sanctuary|PASS||
|379022|Consecration in Flame|PASS||
|379391|Quickened Invocation|PASS||
|380188|Crusader's Resolve|PASS||
|383388|Relentless Inquisitor|PASS||
|384376|Avenging Wrath|PASS||
|384815|Seal of Charity|PASS||
|384820|Sacrifice of the Just|PASS||
|384909|Improved Blessing of Protection|PASS||
|384914|Recompense|PASS||
|385414|Afterimage|PASS||
|385427|Obduracy|PASS||
|385450|Seal of Might|PASS||
|385515|Holy Aegis|PASS||
|385633|Auras of the Resolute|PASS||
|386653|Bulwark of Righteous Fury|PASS||
|386738|Divine Resonance|PASS||
|387791|Empyreal Ward|PASS||
|389539|Sentinel|PASS||
|391054|Intercession|PASS||
|391142|Zealot's Paragon|PASS||
|392928|Tirion's Devotion|PASS||
|393071|Strength in Adversity|PASS||
|393114|Improved Ardent Defender|PASS||
|402964|Sanctified Plates|PASS||
|403530|Punishment|PASS||
|404357|Guided Prayer|PASS||
|406101|Faith's Armor|PASS||
|406468|Lightforged Blessing|PASS||
|425518|Light's Deliverance|PASS||
|427445|Light's Guidance|PASS||
|431377|Dawnlight|PASS||
|431402|Luminosity|PASS||
|431404|Solar Grace|PASS||
|431406|Will of the Dawn|PASS||
|431407|Lingering Radiance|PASS||
|431413|Sun Sear|PASS||
|431423|Illumine|PASS||
|431425|Sun's Avatar|PASS||
|431463|Zealous Vindication|PASS||
|431474|Second Sunrise|PASS||
|431480|Gleaming Rays|PASS||
|431482|Morning Star|PASS||
|431533|Shake the Heavens|PASS||
|431551|Wrathful Descent|PASS||
|431687|Higher Calling|PASS||
|431730|Sacrosanct Crusade|PASS||
|432459|Holy Bulwark|PASS||
|432463|Hammerfall|PASS||
|432615|Endless Wrath|PASS||
|432626|Undisputed Ruling|PASS||
|432802|Solidarity|PASS||
|432804|Forewarning|PASS||
|432821|Shared Resolve|PASS||
|432866|Laying Down Arms|PASS||
|432919|Valiance|PASS||
|432929|Divine Hammer|PASS||
|432964|Divine Inspiration|PASS||
|432977|Sanctification|PASS||
|432990|Unrelenting Charger|PASS||
|432992|Bonds of Fellowship|PASS||
|433011|Blessing of the Forge|PASS||
|433015|Blessed Assurance|PASS||
|433106|Divine Guidance|PASS||
|433568|Rite of Sanctification|PASS||
|433583|Rite of Adjuration|PASS||
|433718|Hammer and Anvil|PASS||
|439760|Aurora|PASS||
|445200|Blessing of An'she|PASS||
|453785|Earthen Ordinant's Ramolith|FAIL|a:effect:77|
|460720|Wrench Evil|PASS||
|462092|Paladin|FAIL|a:aura:320|
|462095|Protection Paladin|PASS||
|469279|Worthy Sacrifice|PASS||
|469304|Steed of Liberty|PASS||
|469309|Eye for an Eye|PASS||
|469316|Stoicism|PASS||
|469317|Stand Against Evil|PASS||
|469321|Righteous Protection|PASS||
|469325|Light's Countenance|PASS||
|469337|Sacred Strength|PASS||
|469409|Divine Spurs|PASS||
|469411|A Just Reward|PASS||
|469416|Lightbearer|PASS||
|469434|Selfless Healer|PASS||
|469439|Inspired Guard|PASS||
|469445|Holy Reprieve|PASS||
|469476|Divine Reach|PASS||
|469701|Tempered in Battle|PASS||
|469770|Blessed Calling|PASS||
|469780|Lead the Charge|PASS||
|469883|Refining Fire|PASS||
|469886|Authoritative Rebuke|PASS||
|1229376|Single-Button Assistant|PASS||
|1241288|Hammer of Wrath|PASS||
|1241945|Blessing of Dusk|PASS||
|1241958|Vengeful Wrath|PASS||
|1244019|Undying Embers|PASS||
|1244070|Searing Sunlight|PASS||
|1245354|Vision of Sanctity|PASS||
|1245891|Solace|PASS||
|1245979|Valiant Crusade|PASS||
|1246481|Empyrean Authority|PASS||
|1246488|Sweeping Verdict|PASS||
|1256936|Protection Paladin|FAIL|a:aura:647; a:aura:648|
|1260429|Divine Exaction|PASS||
|1261525|Light's Judicator|PASS||
|1263252|Seal of the Templar|PASS||
|1263782|Walk Into Light|PASS||
|1263787|Endless Gleam|PASS||
|1263920|Born in Sunlight|PASS||
|1265541|Fear No Evil|PASS||
|1265549|Brought to Light|PASS||
|1271387|Masterwork|PASS||
|1271466|Reflection of Radiance|PASS||
|1271553|Resounding Strike|PASS||
|1276945|Imbued Shield|PASS||
|1277005|Adjudication|PASS||
|1277162|Instrument of the Divine|PASS||
|1301732|Blessed Word|PASS||
|1305063|Protection Paladin|PASS||

### Paladin / Retribution (70)

|Spell|Name|Result|Observed causes|
|---:|---|---|---|
|633|Lay on Hands|FAIL|a:effect:67|
|642|Divine Shield|PASS||
|853|Hammer of Justice|PASS||
|1022|Blessing of Protection|PASS||
|1044|Blessing of Freedom|PASS||
|5502|Sense Undead|PASS||
|6940|Blessing of Sacrifice|FAIL|d:fixture:self_with_exclude_caster|
|7328|Redemption|PASS||
|10326|Turn Evil|FAIL|d:fixture:ineligible_creature_type|
|13819|Summon Warhorse|FAIL|a:effect:77|
|19750|Flash of Light|PASS||
|20271|Judgment|PASS||
|23214|Summon Charger|FAIL|a:effect:77|
|26573|Consecration|PASS||
|31884|Avenging Wrath|PASS||
|34767|Summon Thalassian Charger|FAIL|a:effect:77|
|34769|Summon Thalassian Warhorse|FAIL|a:effect:77|
|35395|Crusader Strike|PASS||
|53385|Divine Storm|PASS||
|53600|Shield of the Righteous|PASS||
|62124|Hand of Reckoning|PASS||
|66235|Ardent Defender|PASS||
|69820|Summon Sunwalker Kodo|FAIL|a:effect:77|
|69826|Summon Great Sunwalker Kodo|FAIL|a:effect:77|
|73629|Summon Exarch's Elekk|FAIL|a:effect:77|
|73630|Summon Great Exarch's Elekk|FAIL|a:effect:77|
|81297|Consecration|PASS||
|82242|Parry|PASS||
|85256|Templar's Verdict|PASS||
|85673|Word of Glory|PASS||
|86539|Plate Specialization|FAIL|d:fixture:no_passive_aura|
|86678|Light of the Ancient Kings|PASS||
|86704|Ancient Fury|PASS||
|94289|Protector of the Innocent|PASS||
|96231|Rebuke|PASS||
|114154|Unbreakable Spirit|PASS||
|115675|Boundless Conviction|PASS||
|115750|Blinding Light|PASS||
|121183|Contemplation|PASS||
|123830|Block|PASS||
|137026|Paladin|PASS||
|137027|Retribution Paladin|PASS||
|146956|Light's Revocation|PASS||
|156322|Eternal Flame|PASS||
|162056|Jailer's Judgment|PASS||
|162698|Stat Negation Aura - Strength DPS|PASS||
|183416|Blessing of Dawn|PASS||
|184575|Blade of Justice|PASS||
|190784|Divine Steed|PASS||
|199422|Holy Ritual|PASS||
|204054|Consecrated Ground|PASS||
|213644|Cleanse Toxins|PASS||
|226178|missing|FAIL|d:missing_store|
|230332|Cavalier|PASS||
|231018|missing|FAIL|d:missing_store|
|231663|Greater Judgment|PASS||
|234299|Fist of Justice|PASS||
|255937|Wake of Ashes|FAIL|a:aura:345|
|267316|Mastery: Highlord's Judgment|PASS||
|270562|Summon Darkforge Ram|FAIL|a:effect:77|
|270564|Summon Dawnforge Ram|FAIL|a:effect:77|
|290608|Crusader's Direhorn|FAIL|a:effect:77|
|305394|Unbound Freedom|PASS||
|315867|Judgment|PASS||
|326732|Empyrean Power|PASS||
|326734|Healing Hands|PASS||
|327977|Judgment|PASS||
|342348|Crusader Strike|PASS||
|343527|Execution Sentence|PASS||
|363613|Summon Lightforged Ruinstrider|FAIL|a:effect:77|
|375576|Divine Toll|PASS||
|376996|Seasoned Warhorse|PASS||
|377128|Golden Path|PASS||
|379391|Quickened Invocation|PASS||
|382536|Sanctify|PASS||
|383228|Swift Justice|FAIL|a:aura:205|
|383328|Final Verdict|PASS||
|383342|Holy Blade|PASS||
|383344|Expurgation|PASS||
|383396|Tempest of the Lightbringer|PASS||
|384027|Divine Resonance|PASS||
|384376|Avenging Wrath|PASS||
|384820|Sacrifice of the Just|PASS||
|384909|Improved Blessing of Protection|PASS||
|384914|Recompense|PASS||
|385414|Afterimage|PASS||
|385427|Obduracy|PASS||
|385450|Seal of Might|PASS||
|385515|Holy Aegis|PASS||
|385633|Auras of the Resolute|PASS||
|387170|Empyrean Legacy|PASS||
|387791|Empyreal Ward|PASS||
|391054|Intercession|PASS||
|402912|Righteous Cause|PASS||
|402964|Sanctified Plates|PASS||
|402971|Jurisdiction|PASS||
|403010|Blessed Champion|PASS||
|403026|Burning Crusade|PASS||
|403479|Lightforged Blessing|PASS||
|403495|Judgment of Justice|PASS||
|403509|Zealot's Fervor|PASS||
|403530|Punishment|PASS||
|403664|Blades of Light|PASS||
|403745|Improved Blade of Justice|PASS||
|403826|Blade of Vengeance|PASS||
|403876|Divine Protection|PASS||
|404357|Guided Prayer|PASS||
|404436|Light of Justice|PASS||
|404512|Highlord's Wrath|PASS||
|404542|Crusading Strikes|FAIL|a:aura:361|
|404752|Instrument of Retribution|PASS||
|404834|Consecrated Blade|PASS||
|405278|Boundless Judgment|PASS||
|405355|Seething Flames|PASS||
|405461|Improved Judgment|PASS||
|406064|Art of War|PASS||
|406101|Faith's Armor|PASS||
|406154|Heart of the Crusader|PASS||
|406157|Judge, Jury and Executioner|PASS||
|406545|Holy Flames|PASS||
|406646|Templar Strikes|PASS||
|406872|Divine Wrath|PASS||
|407067|Rush of Light|PASS||
|408459|Divine Purpose|PASS||
|412314|Retribution Paladin|FAIL|a:aura:646; a:aura:647; a:aura:648; a:aura:649|
|425518|Light's Deliverance|PASS||
|427445|Light's Guidance|PASS||
|431377|Dawnlight|PASS||
|431402|Luminosity|PASS||
|431404|Solar Grace|PASS||
|431406|Will of the Dawn|PASS||
|431407|Lingering Radiance|PASS||
|431413|Sun Sear|PASS||
|431423|Illumine|PASS||
|431425|Sun's Avatar|PASS||
|431463|Zealous Vindication|PASS||
|431474|Second Sunrise|PASS||
|431480|Gleaming Rays|PASS||
|431482|Morning Star|PASS||
|431533|Shake the Heavens|PASS||
|431551|Wrathful Descent|PASS||
|431687|Higher Calling|PASS||
|431730|Sacrosanct Crusade|PASS||
|432463|Hammerfall|PASS||
|432615|Endless Wrath|PASS||
|432626|Undisputed Ruling|PASS||
|432929|Divine Hammer|PASS||
|432977|Sanctification|PASS||
|432990|Unrelenting Charger|PASS||
|432992|Bonds of Fellowship|PASS||
|439760|Aurora|PASS||
|445200|Blessing of An'she|PASS||
|446663|Burn to Ash|PASS||
|453785|Earthen Ordinant's Ramolith|FAIL|a:effect:77|
|458359|Radiant Glory|PASS||
|460720|Wrench Evil|PASS||
|462092|Paladin|FAIL|a:aura:320|
|462096|Retribution Paladin|PASS||
|469279|Worthy Sacrifice|PASS||
|469304|Steed of Liberty|PASS||
|469309|Eye for an Eye|PASS||
|469316|Stoicism|PASS||
|469317|Stand Against Evil|PASS||
|469321|Righteous Protection|PASS||
|469325|Light's Countenance|PASS||
|469337|Sacred Strength|PASS||
|469409|Divine Spurs|PASS||
|469411|A Just Reward|PASS||
|469416|Lightbearer|PASS||
|469434|Selfless Healer|PASS||
|469439|Inspired Guard|PASS||
|469445|Holy Reprieve|PASS||
|469476|Divine Reach|PASS||
|469770|Blessed Calling|PASS||
|469780|Lead the Charge|PASS||
|1229376|Single-Button Assistant|PASS||
|1241288|Hammer of Wrath|PASS||
|1241945|Blessing of Dusk|PASS||
|1241958|Vengeful Wrath|PASS||
|1253598|Crusade|PASS||
|1258011|Retribution Paladin|PASS||
|1260429|Divine Exaction|PASS||
|1261525|Light's Judicator|PASS||
|1261562|Shield of Vengeance|PASS||
|1263252|Seal of the Templar|PASS||
|1263782|Walk Into Light|PASS||
|1263787|Endless Gleam|PASS||
|1263920|Born in Sunlight|PASS||
|1265541|Fear No Evil|PASS||
|1265549|Brought to Light|PASS||

### Warrior / Arms (71)

|Spell|Name|Result|Observed causes|
|---:|---|---|---|
|100|Charge|PASS||
|355|Taunt|PASS||
|772|Rend|PASS||
|845|Cleave|PASS||
|1464|Slam|PASS||
|1680|Whirlwind|PASS||
|1715|Hamstring|PASS||
|2565|Shield Block|PASS||
|3127|Parry|PASS||
|3411|Intervene|FAIL|d:fixture:self_with_exclude_caster|
|5246|Intimidating Shout|PASS||
|6343|Thunder Clap|PASS||
|6544|Heroic Leap|PASS||
|6552|Pummel|PASS||
|6673|Battle Shout|PASS||
|7384|Overpower|PASS||
|12294|Mortal Strike|PASS||
|12323|Piercing Howl|PASS||
|18499|Berserker Rage|PASS||
|23920|Spell Reflection|PASS||
|23922|Shield Slam|PASS||
|29725|Sudden Death|PASS||
|29838|Second Wind|PASS||
|34428|Victory Rush|PASS||
|44949|Whirlwind Off-Hand|PASS||
|46968|Shockwave|FAIL|a:effect:77|
|50622|Bladestorm|PASS||
|57755|Heroic Throw|PASS||
|64382|Shattering Throw|PASS||
|85384|Raging Blow|PASS||
|86101|Plate Specialization|FAIL|d:fixture:no_passive_aura|
|88163|Attack|PASS||
|95738|Bladestorm Off-Hand|PASS||
|96103|Raging Blow|PASS||
|97462|Rallying Cry|PASS||
|103827|Double Time|PASS||
|107570|Storm Bolt|PASS||
|107574|Avatar|PASS||
|113344|Bloodbath|PASS||
|117313|Bloodthirst Heal|PASS||
|118038|Die by the Sword|FAIL|d:fixture:logger_external_consumer:197|
|118779|Victory Rush|PASS||
|123829|Block|PASS||
|137047|Warrior|PASS||
|137049|Arms Warrior|PASS||
|152278|Anger Management|PASS||
|162698|Stat Negation Aura - Strength DPS|PASS||
|163201|Execute|PASS||
|167105|Colossus Smash|PASS||
|184783|Tactician|PASS||
|202163|Bounding Stride|PASS||
|202168|Impending Victory|PASS||
|202316|Fervor of Battle|PASS||
|203201|Crackling Thunder|PASS||
|226178|missing|FAIL|d:missing_store|
|227847|Bladestorm|FAIL|d:fixture:missing_creature_immunities|
|228920|Ravager|PASS||
|231018|missing|FAIL|d:missing_store|
|257191|missing|FAIL|d:missing_store|
|260708|Sweeping Strikes|PASS||
|262150|Dreadnaught|PASS||
|262231|War Machine|PASS||
|275339|Rumbling Earth|PASS||
|279423|Seasoned Soldier|PASS||
|281001|Massacre|PASS||
|294184|Hotbar Slot 01|PASS||
|294189|Hotbar Slot 02|PASS||
|296087|Dual Wield|PASS||
|296306|missing|FAIL|d:missing_store|
|316405|Improved Execute|PASS||
|319158|Victory Rush|PASS||
|334779|Collateral Damage|PASS||
|376079|Champion's Spear|PASS||
|382258|Leeching Strikes|FAIL|a:aura:443|
|382260|Fast Footwork|PASS||
|382549|Pain and Gain|PASS||
|382764|Crushing Force|PASS||
|382767|Overwhelming Rage|PASS||
|382896|Two-Handed Weapon Specialization|PASS||
|382939|Reinforced Plates|PASS||
|382946|Wild Strikes|PASS||
|383082|Barbaric Training|PASS||
|383103|Fueled by Violence|PASS||
|383154|Bloodletting|PASS||
|383287|Bloodborne|PASS||
|383341|Sharpened Blades|PASS||
|383430|Impale|PASS||
|383703|Fatality|PASS||
|384100|Berserker Shout|PASS||
|384110|Wrecking Throw|PASS||
|384124|Armored to the Teeth|PASS||
|384361|Bloodsurge|PASS||
|385571|Improved Overpower|PASS||
|386164|Battle Stance|PASS||
|386208|Defensive Stance|PASS||
|386628|Unhinged|PASS||
|386630|Battlelord|PASS||
|386634|Executioner's Precision|PASS||
|389306|Critical Thinking|PASS||
|391271|Honed Reflexes|PASS||
|392777|Cruel Strikes|PASS||
|392792|Frothing Berserker|PASS||
|400205|Overpowering Finish|PASS||
|400803|Strength of Arms|PASS||
|424742|Battlefield Commander|PASS||
|429634|Colossal Might|PASS||
|429636|Dominance of the Colossus|PASS||
|429637|One Against Many|PASS||
|429638|Martial Expert|FAIL|a:aura:638|
|429639|Boneshaker|PASS||
|429641|Tide of Battle|PASS||
|429642|Mountain of Muscle and Scars|PASS||
|429644|No Stranger to Pain|PASS||
|429647|Practiced Strikes|PASS||
|431548|Precise Might|PASS||
|436358|Demolish|FAIL|d:fixture:missing_creature_immunities|
|440992|Earthquaker|PASS||
|440993|Veteran Vitality|PASS||
|440995|Arterial Bleed|PASS||
|444767|Slayer's Dominance|PASS||
|444769|Imminent Demise|PASS||
|444770|Death Drive|PASS||
|444771|Show No Mercy|PASS||
|444772|Overwhelming Blades|PASS||
|444773|Fierce Followthrough|PASS||
|444774|Opportunist|PASS||
|444775|Reap the Storm|PASS||
|444776|Relentless Pursuit|PASS||
|444777|Vicious Agility|PASS||
|444778|Culling Cyclone|PASS||
|444779|Slayer's Malice|PASS||
|444780|Unrelenting Onslaught|PASS||
|446085|Brutal Finish|PASS||
|462115|Arms Warrior|PASS||
|462116|Warrior|FAIL|a:aura:320|
|1229376|Single-Button Assistant|PASS||
|1243660|Resonant Voice|PASS||
|1244088|Interpose|FAIL|d:fixture:self_with_exclude_caster|
|1256919|Arms Warrior|FAIL|a:aura:647; a:aura:648; a:aura:649|
|1258398|Mastery: Master of Arms|PASS||
|1261046|Brute Force|PASS||
|1261047|Efficiency|PASS||
|1261048|Just Warming Up|PASS||
|1261049|Broad Strokes|PASS||
|1261050|Powerful Momentum|PASS||
|1261051|Tactical Edge|PASS||
|1261056|Crushing Combo|PASS||
|1261057|Master Tactician|PASS||
|1261060|Deep Wounds|PASS||
|1261062|Mortal Wounds|PASS||
|1270704|Decimator|PASS||
|1270709|Cut to the Bone|PASS||
|1270710|Celeritous Conclusion|PASS||
|1270717|Violent Euphoria|PASS||
|1270718|Deadly Focus|PASS||
|1271925|Fearless|PASS||
|1271926|Field Dressing|PASS||
|1271948|Javelineer|PASS||
|1273062|Martial Prowess|PASS||
|1273075|Mass Execution|PASS||
|1277297|Ignore Pain|PASS||
|1280961|Stance Mastery|PASS||

### Warrior / Fury (72)

|Spell|Name|Result|Observed causes|
|---:|---|---|---|
|100|Charge|PASS||
|355|Taunt|PASS||
|1464|Slam|PASS||
|1680|Whirlwind|PASS||
|1715|Hamstring|PASS||
|1719|Recklessness|PASS||
|2565|Shield Block|PASS||
|3127|Parry|PASS||
|3411|Intervene|FAIL|d:fixture:self_with_exclude_caster|
|5246|Intimidating Shout|PASS||
|5308|Execute|PASS||
|6343|Thunder Clap|PASS||
|6544|Heroic Leap|PASS||
|6552|Pummel|PASS||
|6673|Battle Shout|PASS||
|12323|Piercing Howl|PASS||
|12950|Improved Whirlwind|PASS||
|18499|Berserker Rage|PASS||
|23881|Bloodthirst|PASS||
|23920|Spell Reflection|PASS||
|23922|Shield Slam|PASS||
|29725|Sudden Death|PASS||
|29838|Second Wind|PASS||
|34428|Victory Rush|PASS||
|44949|Whirlwind Off-Hand|PASS||
|46917|Titan's Grip|PASS||
|46968|Shockwave|FAIL|a:effect:77|
|50622|Bladestorm|PASS||
|57755|Heroic Throw|PASS||
|64382|Shattering Throw|PASS||
|76856|Mastery: Unshackled Fury|PASS||
|81099|Titan's Grip: Single-Minded Fury|PASS||
|85288|Raging Blow|PASS||
|85384|Raging Blow|PASS||
|86110|Plate Specialization|FAIL|d:fixture:no_passive_aura|
|88163|Attack|PASS||
|95738|Bladestorm Off-Hand|PASS||
|96103|Raging Blow|PASS||
|97462|Rallying Cry|PASS||
|103827|Double Time|PASS||
|107570|Storm Bolt|PASS||
|107574|Avatar|PASS||
|113344|Bloodbath|PASS||
|117313|Bloodthirst Heal|PASS||
|118779|Victory Rush|PASS||
|123829|Block|PASS||
|137047|Warrior|PASS||
|137050|Fury Warrior|PASS||
|152278|Anger Management|PASS||
|162698|Stat Negation Aura - Strength DPS|PASS||
|163201|Execute|PASS||
|184361|Enrage|PASS||
|184364|Enraged Regeneration|PASS||
|184367|Rampage|FAIL|a:aura:443; a:aura:458|
|190411|Whirlwind|PASS||
|202163|Bounding Stride|PASS||
|202168|Impending Victory|PASS||
|203201|Crackling Thunder|PASS||
|206315|Massacre|PASS||
|208154|Warpaint|PASS||
|215568|Fresh Meat|PASS||
|226178|missing|FAIL|d:missing_store|
|227847|Bladestorm|FAIL|d:fixture:missing_creature_immunities|
|231018|missing|FAIL|d:missing_store|
|231842|Dual Wield|FAIL|d:fixture:no_passive_aura|
|257191|missing|FAIL|d:missing_store|
|262231|War Machine|PASS||
|275336|Storm Surge|PASS||
|275339|Rumbling Earth|PASS||
|280392|Meat Cleaver|PASS||
|294184|Hotbar Slot 01|PASS||
|294189|Hotbar Slot 02|PASS||
|296087|Dual Wield|PASS||
|296306|missing|FAIL|d:missing_store|
|316402|Improved Execute|PASS||
|319158|Victory Rush|PASS||
|335077|Frenzy|PASS||
|376079|Champion's Spear|PASS||
|382258|Leeching Strikes|FAIL|a:aura:443|
|382260|Fast Footwork|PASS||
|382549|Pain and Gain|PASS||
|382764|Crushing Force|PASS||
|382767|Overwhelming Rage|PASS||
|382900|Dual Wield Specialization|PASS||
|382939|Reinforced Plates|PASS||
|382946|Wild Strikes|PASS||
|383082|Barbaric Training|PASS||
|383295|Deft Experience|PASS||
|383297|Critical Thinking|PASS||
|383468|Invigorating Fury|PASS||
|383486|Focus in Chaos|PASS||
|383848|Frenzied Enrage|PASS||
|383852|Spite|PASS||
|383854|Improved Raging Blow|PASS||
|383877|Hack and Slash|PASS||
|383885|Vicious Contempt|PASS||
|383959|Cold Steel, Hot Blood|PASS||
|384100|Berserker Shout|PASS||
|384110|Wrecking Throw|PASS||
|384124|Armored to the Teeth|PASS||
|385059|Odyn's Fury|PASS||
|385703|Bloodborne|PASS||
|386196|Berserker Stance|PASS||
|386208|Defensive Stance|PASS||
|386628|Unhinged|PASS||
|391271|Honed Reflexes|PASS||
|392777|Cruel Strikes|PASS||
|392792|Frothing Berserker|PASS||
|392931|Cruelty|PASS||
|392936|Wrath and Fury|PASS||
|393950|Bloodcraze|PASS||
|396749|Reckless Abandon|PASS||
|424742|Battlefield Commander|PASS||
|434969|Lightning Strikes|PASS||
|434970|Steadfast as the Peaks|PASS||
|435607|Thunder Blast|PASS||
|436148|Ground Current|PASS||
|436152|Thorim's Might|PASS||
|436162|Storm Bolts|PASS||
|436201|Gathering Clouds|PASS||
|436707|Crashing Thunder|PASS||
|437068|Strength of the Mountain|PASS||
|437079|Flashing Skies|PASS||
|437118|Burst of Power|PASS||
|437134|Avatar of the Storm|PASS||
|438590|Keep Your Feet on the Ground|PASS||
|438597|Storm Shield|PASS||
|440277|Powerful Enrage|PASS||
|444767|Slayer's Dominance|PASS||
|444769|Imminent Demise|PASS||
|444770|Death Drive|PASS||
|444771|Show No Mercy|PASS||
|444772|Overwhelming Blades|PASS||
|444773|Fierce Followthrough|PASS||
|444774|Opportunist|PASS||
|444775|Reap the Storm|PASS||
|444776|Relentless Pursuit|PASS||
|444777|Vicious Agility|PASS||
|444778|Culling Cyclone|PASS||
|444779|Slayer's Malice|PASS||
|444780|Unrelenting Onslaught|PASS||
|446085|Brutal Finish|PASS||
|456270|Snap Induction|PASS||
|462116|Warrior|FAIL|a:aura:320|
|462117|Fury Warrior|PASS||
|1229376|Single-Button Assistant|PASS||
|1243660|Resonant Voice|PASS||
|1244088|Interpose|FAIL|d:fixture:self_with_exclude_caster|
|1257520|Fury Warrior|FAIL|a:aura:647; a:aura:649|
|1261060|Deep Wounds|PASS||
|1265355|Scent of Blood|PASS||
|1265356|Ragedrinker|PASS||
|1265357|Rampaging Ruin|PASS||
|1265359|Surge of Adrenaline|PASS||
|1265361|Kill or Be Killed|PASS||
|1265570|Executioner's Wrath|PASS||
|1270717|Violent Euphoria|PASS||
|1270718|Deadly Focus|PASS||
|1270723|Conductivity|PASS||
|1270724|Capacitance|PASS||
|1271925|Fearless|PASS||
|1271926|Field Dressing|PASS||
|1271948|Javelineer|PASS||
|1280961|Stance Mastery|PASS||
|1299025|Storm of Blood|PASS||
|1300463|Carving Blades|PASS||

### Warrior / Protection (73)

|Spell|Name|Result|Observed causes|
|---:|---|---|---|
|71|Vanguard|PASS||
|100|Charge|PASS||
|355|Taunt|PASS||
|871|Shield Wall|PASS||
|1160|Demoralizing Shout|PASS||
|1161|Challenging Shout|PASS||
|1464|Slam|PASS||
|1680|Whirlwind|PASS||
|1715|Hamstring|PASS||
|2565|Shield Block|PASS||
|3127|Parry|PASS||
|3411|Intervene|FAIL|d:fixture:self_with_exclude_caster|
|5246|Intimidating Shout|PASS||
|5301|Revenge Trigger|PASS||
|6343|Thunder Clap|PASS||
|6544|Heroic Leap|PASS||
|6552|Pummel|PASS||
|6572|Revenge|PASS||
|6673|Battle Shout|PASS||
|12323|Piercing Howl|PASS||
|18499|Berserker Rage|PASS||
|20243|Devastate|PASS||
|23920|Spell Reflection|PASS||
|23922|Shield Slam|PASS||
|29725|Sudden Death|PASS||
|29838|Second Wind|PASS||
|34428|Victory Rush|PASS||
|44949|Whirlwind Off-Hand|PASS||
|46968|Shockwave|FAIL|a:effect:77|
|50622|Bladestorm|PASS||
|57755|Heroic Throw|PASS||
|64382|Shattering Throw|PASS||
|76857|Mastery: Critical Block|FAIL|a:aura:253|
|85384|Raging Blow|PASS||
|86535|Plate Specialization|FAIL|d:fixture:no_passive_aura|
|88163|Attack|PASS||
|95738|Bladestorm Off-Hand|PASS||
|96103|Raging Blow|PASS||
|97462|Rallying Cry|PASS||
|103827|Double Time|PASS||
|107570|Storm Bolt|PASS||
|107574|Avatar|PASS||
|113344|Bloodbath|PASS||
|117313|Bloodthirst Heal|PASS||
|118779|Victory Rush|PASS||
|123829|Block|PASS||
|137047|Warrior|PASS||
|137048|Protection Warrior|PASS||
|152278|Anger Management|PASS||
|161798|Riposte|PASS||
|162702|Stat Negation Aura - Strength Tank|PASS||
|163201|Execute|PASS||
|190456|Ignore Pain|PASS||
|202095|Indomitable|PASS||
|202163|Bounding Stride|PASS||
|202168|Impending Victory|PASS||
|202560|Best Served Cold|PASS||
|202603|Into the Fray|PASS||
|202743|Booming Voice|PASS||
|203177|Heavy Repercussions|PASS||
|203201|Crackling Thunder|PASS||
|226178|missing|FAIL|d:missing_store|
|228920|Ravager|PASS||
|231018|missing|FAIL|d:missing_store|
|231847|Shield Block|PASS||
|236279|Devastator|PASS||
|257191|missing|FAIL|d:missing_store|
|262231|War Machine|PASS||
|275334|Punish|PASS||
|275336|Storm Surge|PASS||
|275339|Rumbling Earth|PASS||
|281001|Massacre|PASS||
|294184|Hotbar Slot 01|PASS||
|294189|Hotbar Slot 02|PASS||
|296087|Dual Wield|PASS||
|296306|missing|FAIL|d:missing_store|
|319158|Victory Rush|PASS||
|376079|Champion's Spear|PASS||
|382258|Leeching Strikes|FAIL|a:aura:443|
|382260|Fast Footwork|PASS||
|382549|Pain and Gain|PASS||
|382764|Crushing Force|PASS||
|382767|Overwhelming Rage|PASS||
|382895|One-Handed Weapon Specialization|FAIL|a:aura:443|
|382939|Reinforced Plates|PASS||
|382946|Wild Strikes|PASS||
|383082|Barbaric Training|PASS||
|383103|Fueled by Violence|PASS||
|384036|Brutal Vitality|PASS||
|384041|Strategist|PASS||
|384063|Enduring Alacrity|PASS||
|384067|Focused Vigor|PASS||
|384072|Impenetrable Wall|PASS||
|384100|Berserker Shout|PASS||
|384110|Wrecking Throw|PASS||
|384124|Armored to the Teeth|PASS||
|384277|Blood and Thunder|PASS||
|384361|Bloodsurge|PASS||
|385704|Bloodborne|PASS||
|385840|Thunderlord|PASS||
|385888|Tough as Nails|PASS||
|385952|Shield Charge|PASS||
|386011|Shield Specialization|FAIL|a:aura:272|
|386027|Enduring Defenses|PASS||
|386030|Brace For Impact|PASS||
|386071|Disrupting Shout|PASS||
|386164|Battle Stance|PASS||
|386208|Defensive Stance|PASS||
|386394|Battle-Scarred Veteran|PASS||
|386477|Violent Outburst|PASS||
|391271|Honed Reflexes|PASS||
|392777|Cruel Strikes|PASS||
|392792|Frothing Berserker|PASS||
|394311|Instigate|PASS||
|397103|Defender's Aegis|PASS||
|397708|Improved Execute|PASS||
|424742|Battlefield Commander|PASS||
|429634|Colossal Might|PASS||
|429636|Dominance of the Colossus|PASS||
|429637|One Against Many|PASS||
|429638|Martial Expert|FAIL|a:aura:638|
|429639|Boneshaker|PASS||
|429641|Tide of Battle|PASS||
|429642|Mountain of Muscle and Scars|PASS||
|429644|No Stranger to Pain|PASS||
|429647|Practiced Strikes|PASS||
|431548|Precise Might|PASS||
|434969|Lightning Strikes|PASS||
|434970|Steadfast as the Peaks|PASS||
|435607|Thunder Blast|PASS||
|436148|Ground Current|PASS||
|436152|Thorim's Might|PASS||
|436162|Storm Bolts|PASS||
|436201|Gathering Clouds|PASS||
|436358|Demolish|FAIL|d:fixture:missing_creature_immunities|
|436707|Crashing Thunder|PASS||
|437068|Strength of the Mountain|PASS||
|437079|Flashing Skies|PASS||
|437118|Burst of Power|PASS||
|437134|Avatar of the Storm|PASS||
|438590|Keep Your Feet on the Ground|PASS||
|438597|Storm Shield|PASS||
|440992|Earthquaker|PASS||
|440993|Veteran Vitality|PASS||
|440995|Arterial Bleed|PASS||
|452494|Fight Through the Flames|PASS||
|456270|Snap Induction|PASS||
|462116|Warrior|FAIL|a:aura:320|
|462119|Protection Warrior|PASS||
|1229376|Single-Button Assistant|PASS||
|1234769|Armor Specialization|PASS||
|1235022|Hunker Down|PASS||
|1235023|Spellbreaker|PASS||
|1235047|Unyielding Stance|PASS||
|1235088|Heavy Handed|PASS||
|1235113|Whirling Blade|PASS||
|1243659|Last Stand|PASS||
|1243660|Resonant Voice|PASS||
|1256935|Protection Warrior|FAIL|a:aura:647|
|1258163|Protection Warrior|PASS||
|1261060|Deep Wounds|PASS||
|1264330|Devastating Focus|PASS||
|1270704|Decimator|PASS||
|1270709|Cut to the Bone|PASS||
|1270710|Celeritous Conclusion|PASS||
|1270723|Conductivity|PASS||
|1270724|Capacitance|PASS||
|1271925|Fearless|PASS||
|1271926|Field Dressing|PASS||
|1271948|Javelineer|PASS||
|1280961|Stance Mastery|PASS||

### Druid / Balance (102)

|Spell|Name|Result|Observed causes|
|---:|---|---|---|
|99|Incapacitating Roar|PASS||
|339|Entangling Roots|PASS||
|768|Cat Form|PASS||
|774|Rejuvenation|PASS||
|783|Travel Form|PASS||
|1079|Rip|PASS||
|1126|Mark of the Wild|PASS||
|1178|Bear Form Passive|PASS||
|1822|Rake|PASS||
|1850|Dash|PASS||
|2637|Hibernate|FAIL|d:fixture:ineligible_creature_type|
|2782|Remove Corruption|PASS||
|2908|Soothe|PASS||
|3025|Cat Form|PASS||
|5176|Wrath|PASS||
|5211|Mighty Bash|PASS||
|5215|Prowl|PASS||
|5221|Shred|PASS||
|5225|Track Humanoids|PASS||
|5421|Aquatic Form Passive|PASS||
|5487|Bear Form|PASS||
|6795|Growl|PASS||
|8921|Moonfire|PASS||
|8936|Regrowth|PASS||
|16870|Clearcasting|PASS||
|16931|Thick Hide|PASS||
|17057|Bear Form|FAIL|a:aura:94|
|18960|Teleport: Moonglade|PASS||
|20484|Rebirth|PASS||
|20719|Feline Grace|PASS||
|21178|Bear Form Passive 2|PASS||
|22568|Ferocious Bite|FAIL|a:effect:62|
|22570|Maim|PASS||
|22812|Barkskin|PASS||
|22842|Frenzied Regeneration|PASS||
|24858|Moonkin Form|PASS||
|29166|Innervate|PASS||
|33786|Cyclone|PASS||
|33873|Nurturing Instinct|PASS||
|33917|Mangle|PASS||
|37846|Force of Nature|PASS||
|48438|Wild Growth|PASS||
|48503|missing|FAIL|d:missing_store|
|50769|Revive|PASS||
|61391|Typhoon|PASS||
|68484|missing|FAIL|d:missing_store|
|77758|Thrash|PASS||
|78674|Starsurge|PASS||
|78675|Solar Beam|PASS||
|81097|Incarnation: Tree of Life|PASS||
|81098|Incarnation: Tree of Life|PASS||
|81269|Efflorescence|PASS||
|86104|Leather Specialization|FAIL|d:fixture:no_passive_aura|
|88747|Wild Mushroom|PASS||
|93402|Sunfire|PASS||
|102352|Cenarion Ward|PASS||
|102359|Mass Entanglement|PASS||
|102401|Wild Charge|FAIL|d:fixture:OnlyShapeshift:|
|102547|Prowl|PASS||
|102793|Ursol's Vortex|PASS||
|102794|Ursol's Vortex|PASS||
|106898|Stampeding Roar|FAIL|a:effect:77|
|108299|Killer Instinct|PASS||
|114107|Soul of the Forest|PASS||
|114282|Treant Form|PASS||
|125972|Feline Grace|PASS||
|127757|Charm Woodland Creature|FAIL|d:fixture:ineligible_creature_type|
|131768|Feline Swiftness|PASS||
|132469|Typhoon|PASS||
|137009|Druid|FAIL|a:aura:320|
|137013|Balance Druid|PASS||
|145109|Ysera's Gift|PASS||
|145110|Ysera's Gift|FAIL|d:fixture:self_with_exclude_caster|
|157982|Tranquility|PASS||
|159286|Primal Fury|PASS||
|159456|Travel Form|PASS||
|162699|Stat Negation Aura - Intellect DPS|PASS||
|164862|Flap|FAIL|a:aura:483|
|175683|missing|FAIL|d:missing_store|
|190984|Wrath|PASS||
|191034|Starfall|PASS||
|192081|Ironfur|PASS||
|193753|Dreamwalk|FAIL|a:effect:16|
|194153|Starfire|FAIL|a:effect:77|
|197490|Feral Affinity|PASS||
|197524|Astral Influence|PASS||
|197911|Astral Power|PASS||
|202342|Shooting Stars|PASS||
|202345|Starlord|PASS||
|202430|Nature's Balance|PASS||
|202770|Fury of Elune|PASS||
|202918|Light of the Sun|PASS||
|205636|Force of Nature|PASS||
|210053|Mount Form|PASS||
|210065|Track Beasts|PASS||
|211575|missing|FAIL|d:missing_store|
|213764|Swipe|PASS||
|226178|missing|FAIL|d:missing_store|
|228545|Shapeshift Form|PASS||
|231018|missing|FAIL|d:missing_store|
|231040|Lingering Healing|PASS||
|231042|Owlkin Frenzy|PASS||
|231437|Archdruid's Lunarwing Form|PASS||
|236716|Strength of the Wild|PASS||
|252216|Tiger Dash|PASS||
|274281|New Moon|PASS||
|276012|Aquatic Form|PASS||
|276029|Flight Form|PASS||
|279620|Twin Moons|PASS||
|288826|Improved Stampeding Roar|PASS||
|301768|Verdant Heart|PASS||
|326646|Moonfire|PASS||
|327541|Aetherial Kindling|PASS||
|327993|Improved Barkskin|PASS||
|328024|missing|FAIL|d:missing_store|
|343238|Entangling Roots|PASS||
|343647|Solstice|PASS||
|377796|Natural Recovery|PASS||
|377842|Ursine Vigor|PASS||
|377847|Well-Honed Instincts|PASS||
|378988|Lycara's Teachings|PASS||
|383195|Umbral Intensity|PASS||
|383197|Orbit Breaker|PASS||
|385786|Matted Fur|PASS||
|390378|Orbital Strike|PASS||
|391528|Convoke the Spirits|PASS||
|393014|Mastery: Astral Invocation|PASS||
|393760|Umbral Embrace|PASS||
|393940|Starweaver|PASS||
|393954|Rattle the Stars|PASS||
|393991|Elune's Guidance|PASS||
|394013|Incarnation: Chosen of Elune|PASS||
|394046|Power of Goldrinn|PASS||
|394048|Balance of All Things|PASS||
|394065|Denizen of the Dream|PASS||
|394094|Sundered Firmament|PASS||
|394121|Radiant Moonlight|PASS||
|395022|Celestial Alignment|PASS||
|400059|Cosmic Rapidity|PASS||
|400129|Forestwalk|PASS||
|400140|Incessant Tempest|PASS||
|400142|Gale Winds|PASS||
|405834|Improved Prowl|PASS||
|406890|Wild Surges|PASS||
|424058|Boundless Moonlight|PASS||
|424113|The Eternal Moon|PASS||
|428544|Treants of the Moon|PASS||
|428655|The Light of Elune|PASS||
|428731|Harmony of the Grove|PASS||
|428859|Power of Nature|PASS||
|428937|Early Spring|PASS||
|429215|Bounteous Bloom|FAIL|a:aura:382|
|429227|Durability of Nature|FAIL|a:aura:382|
|429399|Expansiveness|PASS||
|429402|Grove's Inspiration|PASS||
|429420|Potent Enchantments|PASS||
|429433|Blooming Infusion|PASS||
|429520|Moon Guardian|PASS||
|429523|Lunar Calling|FAIL|a:aura:220|
|429530|Lunar Insight|PASS||
|429532|Atmospheric Exposure|PASS||
|429533|Glistening Fur|PASS||
|429536|Astral Insight|PASS||
|429538|Moondust|PASS||
|429539|Lunation|PASS||
|429540|Arcane Affinity|PASS||
|429668|Stellar Command|PASS||
|433748|Protective Growth|PASS||
|433831|Dream Surge|PASS||
|434220|Power of the Dream|PASS||
|434249|Control of the Dream|PASS||
|439528|Thriving Growth|PASS||
|439868|Hunt Beneath the Open Skies|PASS||
|439880|Resilient Flourishing|PASS||
|439882|Root Network|PASS||
|439890|Strategic Infusion|PASS||
|439895|Entangling Vortex|PASS||
|439901|Flower Walk|PASS||
|439926|Wildstalker's Power|PASS||
|439929|Bond with Nature|PASS||
|440116|Harmonious Constitution|PASS||
|440117|Twin Sprouts|PASS||
|440118|Implant|PASS||
|440119|Vigorous Creepers|PASS||
|440120|Bursting Growth|PASS||
|441583|Ravage|PASS||
|441675|Fount of Strength|PASS||
|441678|Wildshape Mastery|PASS||
|441689|Empowered Shapeshifting|PASS||
|441691|Wildpower Surge|PASS||
|441809|Dreadful Wound|PASS||
|441814|Ruthless Aggression|PASS||
|441824|Killing Strikes|PASS||
|441829|Aggravate Wounds|PASS||
|441835|Claw Rampage|PASS||
|441841|Bestial Strength|PASS||
|441844|Pack's Endurance|PASS||
|441845|Strike for the Heart|PASS||
|441846|Tear Down the Mighty|PASS||
|443046|Elune's Grace|PASS||
|449182|Ursoc's Spirit|PASS||
|449184|Instincts of the Claw|PASS||
|449185|Lore of the Grove|PASS||
|449191|Oakskin|PASS||
|449193|Fluid Form|PASS||
|450212|Stellar Amplification|PASS||
|450347|Nature's Grace|PASS||
|450356|Touch the Cosmos|PASS||
|450558|Harmony of the Heavens|PASS||
|450598|Astral Communion|PASS||
|451211|Starlight Conduit|PASS||
|455461|Lethal Preservation|PASS||
|455797|Cenarius' Might|PASS||
|462068|Balance Druid|PASS||
|462069|Druid|FAIL|d:fixture:no_passive_aura|
|468743|Whirling Stars|PASS||
|468936|Sunseeker Mushroom|PASS||
|469004|Hail of Stars|PASS||
|474526|Grievous Wounds|PASS||
|474530|Circle of the Wild|PASS||
|474541|Circle of the Heavens|PASS||
|474678|Aessina's Renewal|PASS||
|474750|Symbiotic Relationship|FAIL|d:fixture:self_with_exclude_caster|
|1213597|Perfectly-Honed Instincts|PASS||
|1229376|Single-Button Assistant|PASS||
|1232897|Lycara's Inspiration|PASS||
|1239669|Eclipse|PASS||
|1240185|Celestial Fire|PASS||
|1240188|Sculpt the Stars|PASS||
|1240206|Total Eclipse|PASS||
|1240262|Meteor Storm|PASS||
|1240283|Elune's Challenge|PASS||
|1240906|Improved Eclipse|PASS||
|1240907|Meteorites|PASS||
|1256996|Balance Druid|FAIL|a:aura:647|
|1261867|Heart of the Wild|PASS||
|1262034|Gift of the Wild|PASS||
|1264614|Sylvan Beckoning|PASS||
|1264776|Dryad's Dance|PASS||
|1264899|Spirit of the Thicket|PASS||
|1270565|Green Thumb|PASS||
|1270586|Rampancy|PASS||
|1270592|Patient Custodian|PASS||
|1271206|Star Cascade|PASS||
|1271261|Penumbral Swell|PASS||
|1271305|Bask in Moonlight|PASS||
|1271540|Limb from Limb|PASS||
|1271635|Twin Claw|PASS||
|1271839|Exacerbating Wounds|PASS||
|1297521|Stellar Protection|PASS||

### Druid / Feral (103)

|Spell|Name|Result|Observed causes|
|---:|---|---|---|
|99|Incapacitating Roar|PASS||
|339|Entangling Roots|PASS||
|768|Cat Form|PASS||
|774|Rejuvenation|PASS||
|783|Travel Form|PASS||
|1079|Rip|PASS||
|1126|Mark of the Wild|PASS||
|1178|Bear Form Passive|PASS||
|1822|Rake|PASS||
|1850|Dash|PASS||
|2637|Hibernate|FAIL|d:fixture:ineligible_creature_type|
|2782|Remove Corruption|PASS||
|2908|Soothe|PASS||
|3025|Cat Form|PASS||
|5176|Wrath|PASS||
|5211|Mighty Bash|PASS||
|5215|Prowl|PASS||
|5217|Tiger's Fury|PASS||
|5221|Shred|PASS||
|5225|Track Humanoids|PASS||
|5421|Aquatic Form Passive|PASS||
|5487|Bear Form|PASS||
|6795|Growl|PASS||
|8921|Moonfire|PASS||
|8936|Regrowth|PASS||
|16864|Omen of Clarity|PASS||
|16870|Clearcasting|PASS||
|16931|Thick Hide|PASS||
|16949|Feral Instinct|PASS||
|16974|Predatory Swiftness|PASS||
|17057|Bear Form|FAIL|a:aura:94|
|18960|Teleport: Moonglade|PASS||
|20484|Rebirth|PASS||
|20719|Feline Grace|PASS||
|21178|Bear Form Passive 2|PASS||
|22568|Ferocious Bite|FAIL|a:effect:62|
|22570|Maim|PASS||
|22812|Barkskin|PASS||
|22842|Frenzied Regeneration|PASS||
|24858|Moonkin Form|PASS||
|29166|Innervate|PASS||
|33786|Cyclone|PASS||
|33873|Nurturing Instinct|PASS||
|33917|Mangle|PASS||
|37846|Force of Nature|PASS||
|48438|Wild Growth|PASS||
|48484|Infected Wounds|PASS||
|48503|missing|FAIL|d:missing_store|
|50769|Revive|PASS||
|61336|Survival Instincts|PASS||
|61391|Typhoon|PASS||
|68484|missing|FAIL|d:missing_store|
|77493|Mastery: Razor Claws|PASS||
|77758|Thrash|PASS||
|81097|Incarnation: Tree of Life|PASS||
|81098|Incarnation: Tree of Life|PASS||
|81269|Efflorescence|PASS||
|86097|Leather Specialization|FAIL|d:fixture:no_passive_aura|
|93402|Sunfire|PASS||
|102352|Cenarion Ward|PASS||
|102359|Mass Entanglement|PASS||
|102401|Wild Charge|FAIL|d:fixture:OnlyShapeshift:|
|102543|Incarnation: Avatar of Ashamane|PASS||
|102547|Prowl|PASS||
|102793|Ursol's Vortex|PASS||
|102794|Ursol's Vortex|PASS||
|106839|Skull Bash|PASS||
|106898|Stampeding Roar|FAIL|a:effect:77|
|108299|Killer Instinct|PASS||
|114282|Treant Form|PASS||
|125972|Feline Grace|PASS||
|127757|Charm Woodland Creature|FAIL|d:fixture:ineligible_creature_type|
|131768|Feline Swiftness|PASS||
|132469|Typhoon|PASS||
|137009|Druid|FAIL|a:aura:320|
|137011|Feral Druid|PASS||
|145109|Ysera's Gift|PASS||
|145110|Ysera's Gift|FAIL|d:fixture:self_with_exclude_caster|
|155580|Lunar Inspiration|PASS||
|157447|Critical Strikes|PASS||
|157982|Tranquility|PASS||
|158476|Soul of the Forest|PASS||
|159286|Primal Fury|PASS||
|159456|Travel Form|PASS||
|162697|Stat Negation Aura - Agility DPS|PASS||
|164862|Flap|FAIL|a:aura:483|
|175683|missing|FAIL|d:missing_store|
|192081|Ironfur|PASS||
|193753|Dreamwalk|FAIL|a:effect:16|
|197524|Astral Influence|PASS||
|197626|Starsurge|PASS||
|197628|Starfire|PASS||
|197692|Feral Overrides Passive|PASS||
|202021|Predator|PASS||
|202031|Sabertooth|PASS||
|210053|Mount Form|PASS||
|210065|Track Beasts|PASS||
|211575|missing|FAIL|d:missing_store|
|213764|Swipe|PASS||
|226178|missing|FAIL|d:missing_store|
|228545|Shapeshift Form|PASS||
|231018|missing|FAIL|d:missing_store|
|231040|Lingering Healing|PASS||
|231063|Merciless Claws|PASS||
|231437|Archdruid's Lunarwing Form|PASS||
|236068|Moment of Clarity|PASS||
|236716|Strength of the Wild|PASS||
|252216|Tiger Dash|PASS||
|274837|Feral Frenzy|PASS||
|276012|Aquatic Form|PASS||
|276029|Flight Form|PASS||
|285381|Primal Wrath|PASS||
|288826|Improved Stampeding Roar|PASS||
|300349|Feline Adept|PASS||
|301768|Verdant Heart|PASS||
|327993|Improved Barkskin|PASS||
|328024|missing|FAIL|d:missing_store|
|343223|Berserk|PASS||
|343238|Entangling Roots|PASS||
|377796|Natural Recovery|PASS||
|377842|Ursine Vigor|PASS||
|377847|Well-Honed Instincts|PASS||
|378988|Lycara's Teachings|PASS||
|383352|Tireless Energy|PASS||
|384665|Taste for Blood|PASS||
|384667|Sudden Ambush|PASS||
|385786|Matted Fur|PASS||
|390772|Pouncing Strikes|FAIL|d:fixture:no_passive_aura|
|390864|Wild Slashes|PASS||
|390902|Carnivorous Instinct|PASS||
|391045|Dreadful Bleeding|PASS||
|391078|Raging Fury|PASS||
|391174|Berserk: Heart of the Lion|PASS||
|391347|Rip and Tear|PASS||
|391528|Convoke the Spirits|PASS||
|391548|Ashamane's Guidance|FAIL|d:fixture:no_passive_aura|
|391700|Double-Clawed Rake|PASS||
|391709|Rampant Ferocity|PASS||
|391872|Tiger's Tenacity|PASS||
|391875|Frantic Momentum|PASS||
|391881|Apex Predator's Craving|PASS||
|391978|Veinripper|PASS||
|400129|Forestwalk|PASS||
|400140|Incessant Tempest|PASS||
|400142|Gale Winds|PASS||
|400320|Circle of Life and Death|PASS||
|421432|Saber Jaws|PASS||
|428544|Treants of the Moon|PASS||
|428731|Harmony of the Grove|PASS||
|428859|Power of Nature|PASS||
|428937|Early Spring|PASS||
|429215|Bounteous Bloom|FAIL|a:aura:382|
|429227|Durability of Nature|FAIL|a:aura:382|
|429399|Expansiveness|PASS||
|429402|Grove's Inspiration|PASS||
|429420|Potent Enchantments|PASS||
|429433|Blooming Infusion|PASS||
|433748|Protective Growth|PASS||
|433831|Dream Surge|PASS||
|434220|Power of the Dream|PASS||
|434249|Control of the Dream|PASS||
|439528|Thriving Growth|PASS||
|439868|Hunt Beneath the Open Skies|PASS||
|439880|Resilient Flourishing|PASS||
|439882|Root Network|PASS||
|439890|Strategic Infusion|PASS||
|439895|Entangling Vortex|PASS||
|439901|Flower Walk|PASS||
|439926|Wildstalker's Power|PASS||
|439929|Bond with Nature|PASS||
|440116|Harmonious Constitution|PASS||
|440117|Twin Sprouts|PASS||
|440118|Implant|PASS||
|440119|Vigorous Creepers|PASS||
|440120|Bursting Growth|PASS||
|441583|Ravage|PASS||
|441675|Fount of Strength|PASS||
|441678|Wildshape Mastery|PASS||
|441689|Empowered Shapeshifting|PASS||
|441691|Wildpower Surge|PASS||
|441809|Dreadful Wound|PASS||
|441814|Ruthless Aggression|PASS||
|441824|Killing Strikes|PASS||
|441829|Aggravate Wounds|PASS||
|441835|Claw Rampage|PASS||
|441841|Bestial Strength|PASS||
|441844|Pack's Endurance|PASS||
|441845|Strike for the Heart|PASS||
|441846|Tear Down the Mighty|PASS||
|449182|Ursoc's Spirit|PASS||
|449184|Instincts of the Claw|PASS||
|449185|Lore of the Grove|PASS||
|449191|Oakskin|PASS||
|449193|Fluid Form|PASS||
|449537|Coiled to Spring|PASS||
|449645|Savage Fury|PASS||
|451211|Starlight Conduit|PASS||
|455461|Lethal Preservation|PASS||
|455797|Cenarius' Might|PASS||
|462069|Druid|FAIL|d:fixture:no_passive_aura|
|462070|Feral Druid|PASS||
|474526|Grievous Wounds|PASS||
|474530|Circle of the Wild|PASS||
|474541|Circle of the Heavens|PASS||
|474678|Aessina's Renewal|PASS||
|474750|Symbiotic Relationship|FAIL|d:fixture:self_with_exclude_caster|
|1213597|Perfectly-Honed Instincts|PASS||
|1229376|Single-Button Assistant|PASS||
|1232897|Lycara's Inspiration|PASS||
|1243807|Frantic Frenzy|PASS||
|1244258|Chomp|FAIL|d:fixture:InvalidTarget:SPELL_FAILED_CASTER_AURASTATE|
|1244532|Blood Spattered|PASS||
|1244544|Focused Frenzy|PASS||
|1244547|Hunger for Battle|PASS||
|1244632|Lacerating Claws|PASS||
|1249752|Waiting for Energy|PASS||
|1256992|Feral Druid|FAIL|a:aura:647; a:aura:648|
|1261867|Heart of the Wild|PASS||
|1262034|Gift of the Wild|PASS||
|1264614|Sylvan Beckoning|PASS||
|1264776|Dryad's Dance|PASS||
|1264899|Spirit of the Thicket|PASS||
|1270565|Green Thumb|PASS||
|1270586|Rampancy|PASS||
|1270592|Patient Custodian|PASS||
|1271540|Limb from Limb|PASS||
|1271635|Twin Claw|PASS||
|1271839|Exacerbating Wounds|PASS||
|1280316|Panther's Guile|PASS||

### Druid / Guardian (104)

|Spell|Name|Result|Observed causes|
|---:|---|---|---|
|99|Incapacitating Roar|PASS||
|339|Entangling Roots|PASS||
|768|Cat Form|PASS||
|774|Rejuvenation|PASS||
|783|Travel Form|PASS||
|1079|Rip|PASS||
|1126|Mark of the Wild|PASS||
|1178|Bear Form Passive|PASS||
|1822|Rake|PASS||
|1850|Dash|PASS||
|2637|Hibernate|FAIL|d:fixture:ineligible_creature_type|
|2782|Remove Corruption|PASS||
|2908|Soothe|PASS||
|3025|Cat Form|PASS||
|5176|Wrath|PASS||
|5211|Mighty Bash|PASS||
|5215|Prowl|PASS||
|5221|Shred|PASS||
|5225|Track Humanoids|PASS||
|5421|Aquatic Form Passive|PASS||
|5487|Bear Form|PASS||
|6795|Growl|PASS||
|6807|Maul|PASS||
|8921|Moonfire|PASS||
|8936|Regrowth|PASS||
|16870|Clearcasting|PASS||
|16931|Thick Hide|PASS||
|17057|Bear Form|FAIL|a:aura:94|
|18960|Teleport: Moonglade|PASS||
|20484|Rebirth|PASS||
|20719|Feline Grace|PASS||
|21178|Bear Form Passive 2|PASS||
|22568|Ferocious Bite|FAIL|a:effect:62|
|22570|Maim|PASS||
|22812|Barkskin|PASS||
|22842|Frenzied Regeneration|PASS||
|24858|Moonkin Form|PASS||
|29166|Innervate|PASS||
|33786|Cyclone|PASS||
|33873|Nurturing Instinct|PASS||
|33917|Mangle|PASS||
|37846|Force of Nature|PASS||
|48438|Wild Growth|PASS||
|48503|missing|FAIL|d:missing_store|
|50334|Berserk|FAIL|a:aura:443; d:fixture:missing_creature_immunities|
|50769|Revive|PASS||
|61336|Survival Instincts|PASS||
|61391|Typhoon|PASS||
|68484|missing|FAIL|d:missing_store|
|77758|Thrash|PASS||
|81097|Incarnation: Tree of Life|PASS||
|81098|Incarnation: Tree of Life|PASS||
|81269|Efflorescence|PASS||
|86096|Leather Specialization|FAIL|d:fixture:no_passive_aura|
|93402|Sunfire|PASS||
|102352|Cenarion Ward|PASS||
|102359|Mass Entanglement|PASS||
|102401|Wild Charge|FAIL|d:fixture:OnlyShapeshift:|
|102547|Prowl|PASS||
|102558|Incarnation: Guardian of Ursoc|FAIL|a:aura:443; d:fixture:missing_creature_immunities|
|102793|Ursol's Vortex|PASS||
|102794|Ursol's Vortex|PASS||
|106839|Skull Bash|PASS||
|106898|Stampeding Roar|FAIL|a:effect:77|
|108299|Killer Instinct|PASS||
|114282|Treant Form|PASS||
|125972|Feline Grace|PASS||
|127757|Charm Woodland Creature|FAIL|d:fixture:ineligible_creature_type|
|131768|Feline Swiftness|PASS||
|132469|Typhoon|PASS||
|137009|Druid|FAIL|a:aura:320|
|137010|Guardian Druid|PASS||
|145109|Ysera's Gift|PASS||
|145110|Ysera's Gift|FAIL|d:fixture:self_with_exclude_caster|
|155578|Guardian of Elune|PASS||
|155783|Mastery: Nature's Guardian|PASS||
|155835|Bristling Fur|PASS||
|157447|Critical Strikes|PASS||
|157982|Tranquility|PASS||
|158477|Soul of the Forest|PASS||
|159195|Mastery: Nature's Guardian|PASS||
|159286|Primal Fury|PASS||
|159456|Travel Form|PASS||
|162700|Stat Negation Aura - Agility Tank|PASS||
|164862|Flap|FAIL|a:aura:483|
|175683|missing|FAIL|d:missing_store|
|192081|Ironfur|PASS||
|193753|Dreamwalk|FAIL|a:effect:16|
|197490|Feral Affinity|PASS||
|197524|Astral Influence|PASS||
|197626|Starsurge|PASS||
|197628|Starfire|PASS||
|200854|Gory Fur|PASS||
|203953|Brambles|PASS||
|203962|Blood Frenzy|PASS||
|203964|Galactic Guardian|PASS||
|203965|Survival of the Fittest|PASS||
|204053|Rend and Tear|PASS||
|204066|Lunar Beam|PASS||
|210053|Mount Form|PASS||
|210065|Track Beasts|PASS||
|210706|Gore|PASS||
|211575|missing|FAIL|d:missing_store|
|213764|Swipe|PASS||
|226178|missing|FAIL|d:missing_store|
|228545|Shapeshift Form|PASS||
|231018|missing|FAIL|d:missing_store|
|231040|Lingering Healing|PASS||
|231065|Lightning Reflexes|FAIL|a:aura:198|
|231437|Archdruid's Lunarwing Form|PASS||
|236716|Strength of the Wild|PASS||
|238049|Scintillating Moonlight|PASS||
|252216|Tiger Dash|PASS||
|270100|Bear Form|PASS||
|276012|Aquatic Form|PASS||
|276029|Flight Form|PASS||
|288826|Improved Stampeding Roar|PASS||
|300346|Ursine Adept|PASS||
|301768|Verdant Heart|PASS||
|327993|Improved Barkskin|PASS||
|328024|missing|FAIL|d:missing_store|
|343238|Entangling Roots|PASS||
|345208|Infected Wounds|PASS||
|370586|Elune's Favored|PASS||
|370695|Fury of Nature|PASS||
|371905|After the Wildfire|FAIL|a:aura:285|
|372119|Dream of Cenarius|PASS||
|372567|Twin Moonfire|PASS||
|372618|Vulnerable Flesh|PASS||
|372943|Untamed Savagery|PASS||
|372945|Reinvigoration|PASS||
|377210|Ursoc's Fury|PASS||
|377796|Natural Recovery|PASS||
|377811|Innate Resolve|PASS||
|377835|Front of the Pack|PASS||
|377842|Ursine Vigor|PASS||
|377847|Well-Honed Instincts|PASS||
|378988|Lycara's Teachings|PASS||
|385786|Matted Fur|PASS||
|391528|Convoke the Spirits|PASS||
|393414|Ursoc's Guidance|PASS||
|393427|Flashing Claws|PASS||
|393611|Ursoc's Endurance|PASS||
|393618|Reinforced Fur|PASS||
|400129|Forestwalk|PASS||
|400140|Incessant Tempest|PASS||
|400142|Gale Winds|PASS||
|400254|Raze|PASS||
|400278|Moonless Night|PASS||
|405834|Improved Prowl|PASS||
|424058|Boundless Moonlight|PASS||
|424113|The Eternal Moon|PASS||
|428544|Treants of the Moon|PASS||
|428655|The Light of Elune|PASS||
|428731|Harmony of the Grove|PASS||
|428859|Power of Nature|PASS||
|428937|Early Spring|PASS||
|429215|Bounteous Bloom|FAIL|a:aura:382|
|429227|Durability of Nature|FAIL|a:aura:382|
|429399|Expansiveness|PASS||
|429402|Grove's Inspiration|PASS||
|429420|Potent Enchantments|PASS||
|429433|Blooming Infusion|PASS||
|429520|Moon Guardian|PASS||
|429523|Lunar Calling|FAIL|a:aura:220|
|429530|Lunar Insight|PASS||
|429532|Atmospheric Exposure|PASS||
|429533|Glistening Fur|PASS||
|429536|Astral Insight|PASS||
|429538|Moondust|PASS||
|429539|Lunation|PASS||
|429540|Arcane Affinity|PASS||
|429668|Stellar Command|PASS||
|433748|Protective Growth|PASS||
|433831|Dream Surge|PASS||
|434220|Power of the Dream|PASS||
|434249|Control of the Dream|PASS||
|439528|Thriving Growth|PASS||
|439868|Hunt Beneath the Open Skies|PASS||
|439880|Resilient Flourishing|PASS||
|439882|Root Network|PASS||
|439890|Strategic Infusion|PASS||
|439895|Entangling Vortex|PASS||
|439901|Flower Walk|PASS||
|439926|Wildstalker's Power|PASS||
|439929|Bond with Nature|PASS||
|440116|Harmonious Constitution|PASS||
|440117|Twin Sprouts|PASS||
|440118|Implant|PASS||
|440119|Vigorous Creepers|PASS||
|440120|Bursting Growth|PASS||
|441583|Ravage|PASS||
|441675|Fount of Strength|PASS||
|441678|Wildshape Mastery|PASS||
|441689|Empowered Shapeshifting|PASS||
|441691|Wildpower Surge|PASS||
|441809|Dreadful Wound|PASS||
|441814|Ruthless Aggression|PASS||
|441824|Killing Strikes|PASS||
|441829|Aggravate Wounds|PASS||
|441835|Claw Rampage|PASS||
|441841|Bestial Strength|PASS||
|441844|Pack's Endurance|PASS||
|441845|Strike for the Heart|PASS||
|441846|Tear Down the Mighty|PASS||
|443046|Elune's Grace|PASS||
|449182|Ursoc's Spirit|PASS||
|449184|Instincts of the Claw|PASS||
|449185|Lore of the Grove|PASS||
|449191|Oakskin|PASS||
|449193|Fluid Form|PASS||
|451211|Starlight Conduit|PASS||
|455461|Lethal Preservation|PASS||
|455797|Cenarius' Might|PASS||
|462069|Druid|FAIL|d:fixture:no_passive_aura|
|462071|Guardian Druid|PASS||
|471492|Ursol's Warding|FAIL|a:aura:285|
|474526|Grievous Wounds|PASS||
|474530|Circle of the Wild|PASS||
|474541|Circle of the Heavens|PASS||
|474678|Aessina's Renewal|PASS||
|474750|Symbiotic Relationship|FAIL|d:fixture:self_with_exclude_caster|
|1213597|Perfectly-Honed Instincts|PASS||
|1229376|Single-Button Assistant|PASS||
|1232897|Lycara's Inspiration|PASS||
|1250906|Memory of Ysera|PASS||
|1250923|Ward of the Forest|PASS||
|1251406|Persistence|PASS||
|1251876|Gift of an Ancient Guardian|PASS||
|1252871|Red Moon|PASS||
|1252994|Killing Blow|PASS||
|1253035|Harnessed Rage|PASS||
|1253461|Waking Nightmare|PASS||
|1253799|Sundering Roar|PASS||
|1256993|Guardian Druid|FAIL|a:aura:647|
|1261867|Heart of the Wild|PASS||
|1262034|Gift of the Wild|PASS||
|1264614|Sylvan Beckoning|PASS||
|1264776|Dryad's Dance|PASS||
|1264899|Spirit of the Thicket|PASS||
|1270565|Green Thumb|PASS||
|1270586|Rampancy|PASS||
|1270592|Patient Custodian|PASS||
|1271206|Star Cascade|PASS||
|1271261|Penumbral Swell|PASS||
|1271305|Bask in Moonlight|PASS||
|1271540|Limb from Limb|PASS||
|1271635|Twin Claw|PASS||
|1271839|Exacerbating Wounds|PASS||
|1278789|Natural Resilience|PASS||
|1278886|Dream Guide|PASS||

### Druid / Restoration (105)

|Spell|Name|Result|Observed causes|
|---:|---|---|---|
|99|Incapacitating Roar|PASS||
|339|Entangling Roots|PASS||
|740|Tranquility|FAIL|a:aura:470; d:fixture:missing_creature_immunities|
|768|Cat Form|PASS||
|774|Rejuvenation|PASS||
|783|Travel Form|PASS||
|1079|Rip|PASS||
|1126|Mark of the Wild|PASS||
|1178|Bear Form Passive|PASS||
|1822|Rake|PASS||
|1850|Dash|PASS||
|2637|Hibernate|FAIL|d:fixture:ineligible_creature_type|
|2908|Soothe|PASS||
|3025|Cat Form|PASS||
|5176|Wrath|PASS||
|5211|Mighty Bash|PASS||
|5215|Prowl|PASS||
|5221|Shred|PASS||
|5225|Track Humanoids|PASS||
|5421|Aquatic Form Passive|PASS||
|5487|Bear Form|PASS||
|6795|Growl|PASS||
|8921|Moonfire|PASS||
|8936|Regrowth|PASS||
|16870|Clearcasting|PASS||
|16931|Thick Hide|PASS||
|17057|Bear Form|FAIL|a:aura:94|
|18562|Swiftmend|FAIL|d:fixture:InvalidTarget:SPELL_FAILED_TARGET_AURASTATE|
|18960|Teleport: Moonglade|PASS||
|20484|Rebirth|PASS||
|20719|Feline Grace|PASS||
|21178|Bear Form Passive 2|PASS||
|22568|Ferocious Bite|FAIL|a:effect:62|
|22570|Maim|PASS||
|22812|Barkskin|PASS||
|22842|Frenzied Regeneration|PASS||
|24858|Moonkin Form|PASS||
|29166|Innervate|PASS||
|33763|Lifebloom|PASS||
|33786|Cyclone|PASS||
|33873|Nurturing Instinct|PASS||
|33891|Incarnation: Tree of Life|PASS||
|33917|Mangle|PASS||
|37846|Force of Nature|PASS||
|48438|Wild Growth|PASS||
|48503|missing|FAIL|d:missing_store|
|50769|Revive|PASS||
|61391|Typhoon|PASS||
|68484|missing|FAIL|d:missing_store|
|77495|Mastery: Harmony|PASS||
|77758|Thrash|PASS||
|81097|Incarnation: Tree of Life|PASS||
|81098|Incarnation: Tree of Life|PASS||
|81269|Efflorescence|PASS||
|86093|Leather Specialization|FAIL|d:fixture:no_passive_aura|
|88423|Nature's Cure|PASS||
|93402|Sunfire|PASS||
|102342|Ironbark|PASS||
|102352|Cenarion Ward|PASS||
|102359|Mass Entanglement|PASS||
|102401|Wild Charge|FAIL|d:fixture:OnlyShapeshift:|
|102547|Prowl|PASS||
|102793|Ursol's Vortex|PASS||
|102794|Ursol's Vortex|PASS||
|106898|Stampeding Roar|FAIL|a:effect:77|
|108299|Killer Instinct|PASS||
|113043|Omen of Clarity|PASS||
|114282|Treant Form|PASS||
|125972|Feline Grace|PASS||
|127757|Charm Woodland Creature|FAIL|d:fixture:ineligible_creature_type|
|131768|Feline Swiftness|PASS||
|132158|Nature's Swiftness|PASS||
|132469|Typhoon|PASS||
|137009|Druid|FAIL|a:aura:320|
|137012|Restoration Druid|PASS||
|145108|Ysera's Gift|PASS||
|145109|Ysera's Gift|PASS||
|145110|Ysera's Gift|FAIL|d:fixture:self_with_exclude_caster|
|145205|Efflorescence|PASS||
|155675|Germination|PASS||
|157982|Tranquility|PASS||
|158478|Soul of the Forest|PASS||
|158504|Dream of Cenarius|PASS||
|159286|Primal Fury|PASS||
|159456|Travel Form|PASS||
|162701|Stat Negation Aura - Intellect Healer|PASS||
|164862|Flap|FAIL|a:aura:483|
|175683|missing|FAIL|d:missing_store|
|192081|Ironfur|PASS||
|193753|Dreamwalk|FAIL|a:effect:16|
|197061|Stonebark|PASS||
|197490|Feral Affinity|PASS||
|197524|Astral Influence|PASS||
|197626|Starsurge|PASS||
|197628|Starfire|PASS||
|197721|Flourish|PASS||
|200383|Prosperity|PASS||
|200390|Cultivation|PASS||
|203651|Overgrowth|PASS||
|207383|Abundance|PASS||
|210053|Mount Form|PASS||
|210065|Track Beasts|PASS||
|211575|missing|FAIL|d:missing_store|
|212040|Revitalize|PASS||
|213764|Swipe|PASS||
|226178|missing|FAIL|d:missing_store|
|231018|missing|FAIL|d:missing_store|
|231032|Improved Regrowth|PASS||
|231040|Lingering Healing|PASS||
|231437|Archdruid's Lunarwing Form|PASS||
|236716|Strength of the Wild|PASS||
|252216|Tiger Dash|PASS||
|274902|Photosynthesis|PASS||
|276012|Aquatic Form|PASS||
|276029|Flight Form|PASS||
|288826|Improved Stampeding Roar|PASS||
|289237|Master Shapeshifter|PASS||
|301768|Verdant Heart|PASS||
|327993|Improved Barkskin|PASS||
|328024|missing|FAIL|d:missing_store|
|328025|Improved Wild Growth|PASS||
|343238|Entangling Roots|PASS||
|377796|Natural Recovery|PASS||
|377842|Ursine Vigor|PASS||
|377847|Well-Honed Instincts|PASS||
|378988|Lycara's Teachings|PASS||
|382550|Passing Seasons|PASS||
|382552|Improved Ironbark|PASS||
|382559|Unstoppable Growth|PASS||
|383191|Regenesis|PASS||
|385786|Matted Fur|PASS||
|391528|Convoke the Spirits|PASS||
|392099|Nurturing Dormancy|PASS||
|392116|Regenerative Heartwood|PASS||
|392124|Embrace of the Dream|PASS||
|392220|Flash of Clarity|PASS||
|392221|Waking Dream|FAIL|a:aura:285|
|392256|Harmonious Blooming|PASS||
|392302|Power of the Archdruid|PASS||
|392325|Verdancy|PASS||
|392356|Reforestation|PASS||
|392378|Improved Nature's Cure|PASS||
|392410|Verdant Infusion|PASS||
|393371|Cenarius' Guidance|FAIL|d:fixture:no_passive_aura|
|400129|Forestwalk|PASS||
|400140|Incessant Tempest|PASS||
|400142|Gale Winds|PASS||
|400533|Wild Synthesis|FAIL|a:aura:157|
|403521|Tranquil Mind|PASS||
|404521|Rampant Growth|PASS||
|405834|Improved Prowl|PASS||
|426702|Liveliness|PASS||
|426784|Call of the Elder Druid|PASS||
|428544|Treants of the Moon|PASS||
|428731|Harmony of the Grove|PASS||
|428859|Power of Nature|PASS||
|428937|Early Spring|PASS||
|429215|Bounteous Bloom|FAIL|a:aura:382|
|429227|Durability of Nature|FAIL|a:aura:382|
|429399|Expansiveness|PASS||
|429402|Grove's Inspiration|PASS||
|429420|Potent Enchantments|PASS||
|429433|Blooming Infusion|PASS||
|433748|Protective Growth|PASS||
|433831|Dream Surge|PASS||
|434220|Power of the Dream|PASS||
|434249|Control of the Dream|PASS||
|439528|Thriving Growth|PASS||
|439868|Hunt Beneath the Open Skies|PASS||
|439880|Resilient Flourishing|PASS||
|439882|Root Network|PASS||
|439890|Strategic Infusion|PASS||
|439895|Entangling Vortex|PASS||
|439901|Flower Walk|PASS||
|439926|Wildstalker's Power|PASS||
|439929|Bond with Nature|PASS||
|440015|Remove Corruption|PASS||
|440116|Harmonious Constitution|PASS||
|440117|Twin Sprouts|PASS||
|440118|Implant|PASS||
|440119|Vigorous Creepers|PASS||
|440120|Bursting Growth|PASS||
|441583|Ravage|PASS||
|441675|Fount of Strength|PASS||
|441678|Wildshape Mastery|PASS||
|441689|Empowered Shapeshifting|PASS||
|441691|Wildpower Surge|PASS||
|441809|Dreadful Wound|PASS||
|441814|Ruthless Aggression|PASS||
|441824|Killing Strikes|PASS||
|441829|Aggravate Wounds|PASS||
|441835|Claw Rampage|PASS||
|441841|Bestial Strength|PASS||
|441844|Pack's Endurance|PASS||
|441845|Strike for the Heart|PASS||
|441846|Tear Down the Mighty|PASS||
|447131|Thriving Vegetation|PASS||
|449182|Ursoc's Spirit|PASS||
|449184|Instincts of the Claw|PASS||
|449185|Lore of the Grove|PASS||
|449191|Oakskin|PASS||
|449193|Fluid Form|PASS||
|451211|Starlight Conduit|PASS||
|455461|Lethal Preservation|PASS||
|455797|Cenarius' Might|PASS||
|462069|Druid|FAIL|d:fixture:no_passive_aura|
|462073|Restoration Druid|PASS||
|468146|Reactive Resin|PASS||
|470549|Improved Swiftmend|PASS||
|470562|Renewing Surge|PASS||
|474526|Grievous Wounds|PASS||
|474530|Circle of the Wild|PASS||
|474541|Circle of the Heavens|PASS||
|474678|Aessina's Renewal|PASS||
|474750|Symbiotic Relationship|FAIL|d:fixture:self_with_exclude_caster|
|1213597|Perfectly-Honed Instincts|PASS||
|1217941|Lifetreading|PASS||
|1226140|Grove Guardians|PASS||
|1229376|Single-Button Assistant|PASS||
|1232897|Lycara's Inspiration|PASS||
|1256995|Restoration Druid|FAIL|a:aura:646; a:aura:647|
|1261867|Heart of the Wild|FAIL|d:fixture:InvalidTarget:SPELL_FAILED_CASTER_AURASTATE|
|1262034|Gift of the Wild|PASS||
|1263879|Nature's Bounty|PASS||
|1264614|Sylvan Beckoning|PASS||
|1264649|Intensity|PASS||
|1264776|Dryad's Dance|PASS||
|1264899|Spirit of the Thicket|PASS||
|1270565|Green Thumb|PASS||
|1270586|Rampancy|PASS||
|1270592|Patient Custodian|PASS||
|1271540|Limb from Limb|PASS||
|1271635|Twin Claw|PASS||
|1271839|Exacerbating Wounds|PASS||

### Death Knight / Blood (250)

|Spell|Name|Result|Observed causes|
|---:|---|---|---|
|3714|Path of Frost|PASS||
|43265|Death and Decay|PASS||
|45524|Chains of Ice|PASS||
|46585|Raise Dead|PASS||
|47528|Mind Freeze|PASS||
|47541|Death Coil|FAIL|d:fixture:living_death_coil_self|
|48263|Veteran of the Third War|PASS||
|48265|Death's Advance|FAIL|d:fixture:missing_creature_immunities|
|48707|Anti-Magic Shell|FAIL|a:aura:267; a:aura:329|
|48743|Death Pact|FAIL|d:fixture:self_with_exclude_caster|
|48792|Icebound Fortitude|PASS||
|49028|Dancing Rune Weapon|PASS||
|49039|Lichborne|FAIL|a:aura:443|
|49576|Death Grip|PASS||
|49998|Death Strike|PASS||
|50842|Blood Boil|PASS||
|50977|Death Gate|PASS||
|51052|Anti-Magic Zone|PASS||
|51986|On a Pale Horse|PASS||
|52212|Death and Decay|PASS||
|55078|Blood Plague|FAIL|a:aura:381|
|55095|Frost Fever|FAIL|a:aura:381|
|55233|Vampiric Blood|FAIL|a:aura:443|
|56222|Dark Command|PASS||
|61999|Raise Ally|PASS||
|77513|Mastery: Blood Shield|PASS||
|81136|Crimson Scourge|PASS||
|82246|Parry|PASS||
|86524|missing|FAIL|d:missing_store|
|86537|Plate Specialization|FAIL|d:fixture:no_passive_aura|
|108199|Gorefiend's Grasp|FAIL|a:effect:77|
|111673|Control Undead|FAIL|d:fixture:ineligible_creature_type|
|114556|Purgatory|PASS||
|127344|Corpse Exploder|PASS||
|137005|Death Knight|PASS||
|137008|Blood Death Knight|PASS||
|161797|Riposte|PASS||
|162702|Stat Negation Aura - Strength Tank|PASS||
|175678|missing|FAIL|d:missing_store|
|175680|missing|FAIL|d:missing_store|
|194662|Rapid Decomposition|PASS||
|194878|Icy Talons|PASS||
|195182|Marrowrend|PASS||
|195292|Death's Caress|FAIL|a:aura:381|
|195621|Frost Fever|PASS||
|195679|Bloodworms|PASS||
|195740|missing|FAIL|d:missing_store|
|195761|missing|FAIL|d:missing_store|
|197147|Festering Wound|PASS||
|205723|Red Thirst|PASS||
|205727|Anti-Magic Barrier|PASS||
|206930|Heart Strike|PASS||
|206967|Will of the Necropolis|PASS||
|206974|Foul Bulwark|PASS||
|207104|Runic Attenuation|PASS||
|207167|Blinding Sleet|PASS||
|207200|Permafrost|PASS||
|212552|Wraith Walk|PASS||
|219786|Ossuary|PASS||
|220667|missing|FAIL|d:missing_store|
|221536|Heartbreaker|PASS||
|221562|Asphyxiate|FAIL|a:aura:298|
|226178|missing|FAIL|d:missing_store|
|229767|missing|FAIL|d:missing_store|
|231018|missing|FAIL|d:missing_store|
|255259|missing|FAIL|d:missing_store|
|273946|Hemostasis|PASS||
|273952|Grip of the Dead|PASS||
|273953|Voracious|PASS||
|276079|Death's Reach|PASS||
|278107|Val'kyr|PASS||
|316239|Rune Strike|PASS||
|316916|Cleaving Strikes|PASS||
|317133|Improved Vampiric Blood|PASS||
|317610|Relish in Blood|PASS||
|356367|Death's Echo|PASS||
|373930|Proliferating Chill|PASS||
|374030|Blood Scent|FAIL|a:aura:443|
|374049|Suppression|PASS||
|374261|Unholy Bond|PASS||
|374265|Unholy Momentum|PASS||
|374277|Improved Death Strike|PASS||
|374383|Assimilation|PASS||
|374504|Brittle|PASS||
|374574|Rune Mastery|PASS||
|374598|Blood Draw|PASS||
|374715|Improved Bone Shield|PASS||
|374717|Improved Heart Strike|PASS||
|374721|Blood Fortification|PASS||
|374747|Perseverance of the Ebon Blade|PASS||
|377629|Leeching Strike|PASS||
|377637|Insatiable Blade|PASS||
|377668|Everlasting Bond|PASS||
|378848|Coldthirst|PASS||
|389682|Unholy Endurance|PASS||
|391386|Blood Feast|PASS||
|391395|Iron Heart|PASS||
|391398|Bloodshot|PASS||
|391458|Sanguine Ground|PASS||
|391477|Coagulopathy|PASS||
|391517|Umbilicus Eternus|PASS||
|391546|March of Darkness|PASS||
|391566|Insidious Chill|PASS||
|391571|Gloom Ward|PASS||
|392566|Enfeeble|PASS||
|433901|Vampiric Strike|PASS||
|433934|Newly Turned|PASS||
|434028|Vampiric Speed|PASS||
|434033|Blood-Soaked Ground|PASS||
|434075|Frenzied Bloodthirst|PASS||
|434100|Vampiric Aura|FAIL|a:aura:443|
|434136|Bloody Fortitude|PASS||
|434143|Infliction of Sorrow|PASS||
|434151|Incite Terror|PASS||
|434152|Gift of the San'layn|PASS||
|434157|Visceral Strength|PASS||
|434260|The Blood is Life|PASS||
|434261|Pact of the San'layn|PASS||
|434263|Sanguine Scent|PASS||
|434905|Grim Reaper|PASS||
|436687|Dark Talons|PASS||
|437122|Death's Messenger|PASS||
|437161|Soul Rupture|PASS||
|439843|Reaper's Mark|PASS||
|439851|Wave of Souls|PASS||
|439948|Expelling Shield|PASS||
|440031|Bind in Darkness|FAIL|a:aura:220|
|440282|Rune Carved Plates|PASS||
|440476|Pact of the Deathbringer|PASS||
|441378|Exterminate|PASS||
|441894|Wither Away|PASS||
|443560|Swift and Painful|PASS||
|444005|Rider's Champion|PASS||
|444008|On a Paler Horse|PASS||
|444010|Death Charge|PASS||
|444033|Whitemane's Famine|PASS||
|444037|Hungering Thirst|PASS||
|444040|Apocalypse Now|PASS||
|444047|Mograine's Might|PASS||
|444052|Nazgrim's Conquest|PASS||
|444069|Fury of the Horsemen|PASS||
|444072|A Feast of Souls|PASS||
|444074|Horsemen's Aid|PASS||
|444083|Pact of the Apocalypse|PASS||
|444097|Trollbane's Icy Fury|PASS||
|444099|Mawsworn Menace|PASS||
|454786|Ice Prison|PASS||
|454788|Runic Protection|FAIL|d:fixture:logger_external_consumer:197|
|454822|Subduing Grasp|PASS||
|454835|Osmosis|PASS||
|454842|Null Magic|PASS||
|454851|Vestigial Shell|PASS||
|457574|Unyielding Will|PASS||
|458572|Bone Collector|PASS||
|458752|Carnage|PASS||
|458753|Bloodied Blade|PASS||
|462061|Blood Death Knight|PASS||
|462062|Death Knight|FAIL|a:aura:320|
|469870|Reaper's Onslaught|PASS||
|1229376|Single-Button Assistant|PASS||
|1234559|Desecrate|PASS||
|1256916|Blood Death Knight|FAIL|a:aura:647; a:aura:649|
|1263569|Abomination Limb|FAIL|a:effect:77|
|1263743|Blood Mist|PASS||
|1263781|Sanguinary Burst|PASS||
|1263824|Consumption|PASS||
|1264235|Deadly Reach|PASS||
|1264296|Lifeblood|PASS||
|1265547|Thrill of Blood|PASS||
|1265574|Transfusion|PASS||
|1265790|Boiling Point|PASS||
|1265855|Echoing Fury|PASS||
|1265859|Frigid Resolve|PASS||
|1265869|Plague Infusion|PASS||
|1265932|Deathly Blows|PASS||
|1265949|Let Terror Reign|PASS||
|1265959|Ride or Die!|PASS||
|1265971|Unholy Armaments|FAIL|a:aura:382|
|1266818|Death Defiance|PASS||
|1266819|Death Notes|PASS||
|1267028|Blood Bond|PASS||
|1279633|Bloody Reflection|PASS||
|1280658|Inevitable|PASS||

### Death Knight / Frost (251)

|Spell|Name|Result|Observed causes|
|---:|---|---|---|
|674|Dual Wield|PASS||
|3714|Path of Frost|PASS||
|43265|Death and Decay|PASS||
|45524|Chains of Ice|PASS||
|46585|Raise Dead|PASS||
|47528|Mind Freeze|PASS||
|47541|Death Coil|FAIL|d:fixture:living_death_coil_self|
|47568|Empower Rune Weapon|PASS||
|48263|Veteran of the Third War|PASS||
|48265|Death's Advance|FAIL|d:fixture:missing_creature_immunities|
|48707|Anti-Magic Shell|FAIL|a:aura:267; a:aura:329|
|48743|Death Pact|FAIL|d:fixture:self_with_exclude_caster|
|48792|Icebound Fortitude|PASS||
|49020|Obliterate|PASS||
|49039|Lichborne|FAIL|a:aura:443|
|49143|Frost Strike|PASS||
|49184|Howling Blast|PASS||
|49576|Death Grip|PASS||
|49998|Death Strike|PASS||
|50977|Death Gate|PASS||
|51052|Anti-Magic Zone|PASS||
|51128|Killing Machine|PASS||
|51271|Pillar of Frost|PASS||
|51986|On a Pale Horse|PASS||
|52212|Death and Decay|PASS||
|55078|Blood Plague|FAIL|a:aura:381|
|55095|Frost Fever|FAIL|a:aura:381|
|56222|Dark Command|PASS||
|59057|Rime|PASS||
|61999|Raise Ally|PASS||
|77514|Mastery: Frozen Heart|PASS||
|81229|Runic Empowerment|PASS||
|81333|Might of the Frozen Wastes|PASS||
|82246|Parry|PASS||
|86113|Plate Specialization|FAIL|d:fixture:no_passive_aura|
|86524|missing|FAIL|d:missing_store|
|111673|Control Undead|FAIL|d:fixture:ineligible_creature_type|
|127344|Corpse Exploder|PASS||
|137005|Death Knight|PASS||
|137006|Frost Death Knight|PASS||
|162698|Stat Negation Aura - Strength DPS|PASS||
|175678|missing|FAIL|d:missing_store|
|175680|missing|FAIL|d:missing_store|
|178819|Dark Succor|PASS||
|194878|Icy Talons|PASS||
|194912|Gathering Storm|PASS||
|194913|Glacial Advance|PASS||
|195621|Frost Fever|PASS||
|195761|missing|FAIL|d:missing_store|
|196770|Remorseless Winter|PASS||
|197147|Festering Wound|PASS||
|205727|Anti-Magic Barrier|PASS||
|206967|Will of the Necropolis|PASS||
|207057|Shattering Blade|PASS||
|207061|Murderous Efficiency|PASS||
|207104|Runic Attenuation|PASS||
|207142|Avalanche|PASS||
|207167|Blinding Sleet|PASS||
|207200|Permafrost|PASS||
|207230|Frostscythe|PASS||
|212552|Wraith Walk|PASS||
|220667|missing|FAIL|d:missing_store|
|221562|Asphyxiate|FAIL|a:aura:298|
|226178|missing|FAIL|d:missing_store|
|229767|missing|FAIL|d:missing_store|
|231018|missing|FAIL|d:missing_store|
|253593|Inexorable Assault|PASS||
|255259|missing|FAIL|d:missing_store|
|273952|Grip of the Dead|PASS||
|276079|Death's Reach|PASS||
|278107|Val'kyr|PASS||
|279302|Frostwyrm's Fury|PASS||
|281238|Obliteration|PASS||
|316239|Rune Strike|PASS||
|316803|Runic Overflow|PASS||
|316916|Cleaving Strikes|PASS||
|317214|Frostreaper|FAIL|d:fixture:no_passive_aura|
|356367|Death's Echo|PASS||
|373930|Proliferating Chill|PASS||
|374030|Blood Scent|FAIL|a:aura:443|
|374049|Suppression|PASS||
|374261|Unholy Bond|PASS||
|374265|Unholy Momentum|PASS||
|374277|Improved Death Strike|PASS||
|374383|Assimilation|PASS||
|374504|Brittle|PASS||
|374574|Rune Mastery|PASS||
|374598|Blood Draw|PASS||
|376251|Runic Command|PASS||
|376938|Everfrost|PASS||
|377056|Biting Cold|PASS||
|377073|Frigid Executioner|PASS||
|377076|Rage of the Frozen Champion|PASS||
|377098|Bonegrinder|PASS||
|377190|Enduring Strength|PASS||
|377226|Frozen Dominion|PASS||
|378848|Coldthirst|PASS||
|389682|Unholy Endurance|PASS||
|391546|March of Darkness|PASS||
|391566|Insidious Chill|PASS||
|391571|Gloom Ward|PASS||
|392566|Enfeeble|PASS||
|392950|Icebreaker|PASS||
|433901|Vampiric Strike|PASS||
|433934|Newly Turned|PASS||
|434028|Vampiric Speed|PASS||
|434033|Blood-Soaked Ground|PASS||
|434075|Frenzied Bloodthirst|PASS||
|434100|Vampiric Aura|FAIL|a:aura:443|
|434136|Bloody Fortitude|PASS||
|434143|Infliction of Sorrow|PASS||
|434151|Incite Terror|PASS||
|434152|Gift of the San'layn|PASS||
|434157|Visceral Strength|PASS||
|434260|The Blood is Life|PASS||
|434261|Pact of the San'layn|PASS||
|434263|Sanguine Scent|PASS||
|434905|Grim Reaper|PASS||
|435005|Smothering Offense|PASS||
|435010|Icy Death Torrent|PASS||
|436687|Dark Talons|PASS||
|437122|Death's Messenger|PASS||
|437161|Soul Rupture|PASS||
|439843|Reaper's Mark|PASS||
|439851|Wave of Souls|PASS||
|439948|Expelling Shield|PASS||
|440031|Bind in Darkness|FAIL|a:aura:220|
|440282|Rune Carved Plates|PASS||
|440476|Pact of the Deathbringer|PASS||
|441378|Exterminate|PASS||
|441894|Wither Away|PASS||
|443560|Swift and Painful|PASS||
|444005|Rider's Champion|PASS||
|444008|On a Paler Horse|PASS||
|444010|Death Charge|PASS||
|444033|Whitemane's Famine|PASS||
|444037|Hungering Thirst|PASS||
|444040|Apocalypse Now|PASS||
|444047|Mograine's Might|PASS||
|444052|Nazgrim's Conquest|PASS||
|444069|Fury of the Horsemen|PASS||
|444072|A Feast of Souls|PASS||
|444074|Horsemen's Aid|PASS||
|444083|Pact of the Apocalypse|PASS||
|444097|Trollbane's Icy Fury|PASS||
|444099|Mawsworn Menace|PASS||
|454786|Ice Prison|PASS||
|454788|Runic Protection|FAIL|d:fixture:logger_external_consumer:197|
|454822|Subduing Grasp|PASS||
|454835|Osmosis|PASS||
|454842|Null Magic|PASS||
|454851|Vestigial Shell|PASS||
|455993|Frostbane|PASS||
|456230|Arctic Assault|PASS||
|456237|Cryogenic Chamber|PASS||
|456238|Hyperpyrexia|PASS||
|456240|The Long Winter|PASS||
|457574|Unyielding Will|PASS||
|462062|Death Knight|FAIL|a:aura:320|
|462063|Frost Death Knight|PASS||
|469870|Reaper's Onslaught|PASS||
|1229376|Single-Button Assistant|PASS||
|1230153|Killing Streak|PASS||
|1230223|Howling Blades|PASS||
|1230272|Icy Onslaught|PASS||
|1230284|Northwinds|PASS||
|1230301|Frostreaper|PASS||
|1234559|Desecrate|PASS||
|1238680|Frostbound Will|PASS||
|1249658|Breath of Sindragosa|PASS||
|1256915|Frost Death Knight|FAIL|a:aura:647|
|1265547|Thrill of Blood|PASS||
|1265574|Transfusion|PASS||
|1265855|Echoing Fury|PASS||
|1265859|Frigid Resolve|PASS||
|1265932|Deathly Blows|PASS||
|1265949|Let Terror Reign|PASS||
|1265959|Ride or Die!|PASS||
|1265971|Unholy Armaments|FAIL|a:aura:382|
|1266818|Death Defiance|PASS||
|1266819|Death Notes|PASS||
|1267028|Blood Bond|PASS||
|1280658|Inevitable|PASS||

### Death Knight / Unholy (252)

|Spell|Name|Result|Observed causes|
|---:|---|---|---|
|3714|Path of Frost|PASS||
|42650|Army of the Dead|PASS||
|43265|Death and Decay|PASS||
|45524|Chains of Ice|PASS||
|46584|Raise Dead|PASS||
|46585|Raise Dead|PASS||
|47528|Mind Freeze|PASS||
|47541|Death Coil|FAIL|d:fixture:living_death_coil_self|
|48263|Veteran of the Third War|PASS||
|48265|Death's Advance|FAIL|d:fixture:missing_creature_immunities|
|48707|Anti-Magic Shell|FAIL|a:aura:267; a:aura:329|
|48743|Death Pact|FAIL|d:fixture:self_with_exclude_caster|
|48792|Icebound Fortitude|PASS||
|49039|Lichborne|FAIL|a:aura:443|
|49530|Sudden Doom|PASS||
|49576|Death Grip|PASS||
|49998|Death Strike|PASS||
|50977|Death Gate|PASS||
|51052|Anti-Magic Zone|PASS||
|51462|Runic Corruption|PASS||
|51986|On a Pale Horse|PASS||
|52212|Death and Decay|PASS||
|55078|Blood Plague|FAIL|a:aura:381|
|55090|Scourge Strike|PASS||
|55095|Frost Fever|FAIL|a:aura:381|
|56222|Dark Command|PASS||
|61999|Raise Ally|PASS||
|77515|Mastery: Dreadblade|PASS||
|77575|Outbreak|FAIL|a:aura:381|
|82246|Parry|PASS||
|85948|Festering Strike|PASS||
|86524|missing|FAIL|d:missing_store|
|86536|Plate Specialization|FAIL|d:fixture:no_passive_aura|
|111673|Control Undead|FAIL|d:fixture:ineligible_creature_type|
|127344|Corpse Exploder|PASS||
|137005|Death Knight|PASS||
|137007|Unholy Death Knight|PASS||
|162698|Stat Negation Aura - Strength DPS|PASS||
|175678|missing|FAIL|d:missing_store|
|175680|missing|FAIL|d:missing_store|
|178819|Dark Succor|PASS||
|194878|Icy Talons|PASS||
|194916|All Will Serve|PASS||
|195621|Frost Fever|PASS||
|195761|missing|FAIL|d:missing_store|
|197147|Festering Wound|PASS||
|205727|Anti-Magic Barrier|PASS||
|206967|Will of the Necropolis|PASS||
|207104|Runic Attenuation|PASS||
|207167|Blinding Sleet|PASS||
|207200|Permafrost|PASS||
|207269|Ebon Fever|PASS||
|207272|Infected Claws|PASS||
|207317|Epidemic|PASS||
|212552|Wraith Walk|PASS||
|220667|missing|FAIL|d:missing_store|
|221562|Asphyxiate|FAIL|a:aura:298|
|226178|missing|FAIL|d:missing_store|
|229767|missing|FAIL|d:missing_store|
|231018|missing|FAIL|d:missing_store|
|255259|missing|FAIL|d:missing_store|
|273952|Grip of the Dead|PASS||
|276023|Harbinger of Doom|PASS||
|276079|Death's Reach|PASS||
|278107|Val'kyr|PASS||
|288848|Necromancer's Cunning|PASS||
|316239|Rune Strike|PASS||
|316916|Cleaving Strikes|PASS||
|325554|Dark Transformation|PASS||
|343294|Soul Reaper|FAIL|a:aura:381|
|356367|Death's Echo|PASS||
|373930|Proliferating Chill|PASS||
|374030|Blood Scent|FAIL|a:aura:443|
|374049|Suppression|PASS||
|374261|Unholy Bond|PASS||
|374265|Unholy Momentum|PASS||
|374277|Improved Death Strike|PASS||
|374383|Assimilation|PASS||
|374504|Brittle|PASS||
|374574|Rune Mastery|PASS||
|374598|Blood Draw|PASS||
|377440|Unholy Aura|PASS||
|377514|Reaping|PASS||
|377580|Putrid Echoes|PASS||
|377587|Ghoulish Frenzy|PASS||
|377590|Ancient Power|PASS||
|377592|Morbidity|PASS||
|378848|Coldthirst|PASS||
|389682|Unholy Endurance|PASS||
|390166|Plague Mastery|PASS||
|390196|Magus of the Dead|PASS||
|390259|Commander of the Dead|PASS||
|390270|Coil of Devastation|PASS||
|390283|Superstrain|PASS||
|391546|March of Darkness|PASS||
|391566|Insidious Chill|PASS||
|391571|Gloom Ward|PASS||
|392566|Enfeeble|PASS||
|433901|Vampiric Strike|PASS||
|433934|Newly Turned|PASS||
|434028|Vampiric Speed|PASS||
|434033|Blood-Soaked Ground|PASS||
|434075|Frenzied Bloodthirst|PASS||
|434100|Vampiric Aura|FAIL|a:aura:443|
|434136|Bloody Fortitude|PASS||
|434143|Infliction of Sorrow|PASS||
|434151|Incite Terror|PASS||
|434152|Gift of the San'layn|PASS||
|434157|Visceral Strength|PASS||
|434260|The Blood is Life|PASS||
|434261|Pact of the San'layn|PASS||
|434263|Sanguine Scent|PASS||
|434905|Grim Reaper|PASS||
|436687|Dark Talons|PASS||
|437122|Death's Messenger|PASS||
|437161|Soul Rupture|PASS||
|439843|Reaper's Mark|PASS||
|439851|Wave of Souls|PASS||
|439948|Expelling Shield|PASS||
|440031|Bind in Darkness|FAIL|a:aura:220|
|440282|Rune Carved Plates|PASS||
|440476|Pact of the Deathbringer|PASS||
|441378|Exterminate|PASS||
|441894|Wither Away|PASS||
|443560|Swift and Painful|PASS||
|444005|Rider's Champion|PASS||
|444008|On a Paler Horse|PASS||
|444010|Death Charge|PASS||
|444033|Whitemane's Famine|PASS||
|444037|Hungering Thirst|PASS||
|444040|Apocalypse Now|PASS||
|444047|Mograine's Might|PASS||
|444052|Nazgrim's Conquest|PASS||
|444069|Fury of the Horsemen|PASS||
|444072|A Feast of Souls|PASS||
|444074|Horsemen's Aid|PASS||
|444083|Pact of the Apocalypse|PASS||
|444097|Trollbane's Icy Fury|PASS||
|444099|Mawsworn Menace|PASS||
|454786|Ice Prison|PASS||
|454788|Runic Protection|FAIL|d:fixture:logger_external_consumer:197|
|454822|Subduing Grasp|PASS||
|454835|Osmosis|PASS||
|454842|Null Magic|PASS||
|454851|Vestigial Shell|PASS||
|455135|Menacing Magus|PASS||
|455386|Doomed Bidding|PASS||
|455396|Foul Infections|PASS||
|455397|Festering Scythe|PASS||
|457574|Unyielding Will|PASS||
|462062|Death Knight|FAIL|a:aura:320|
|462064|Unholy Death Knight|PASS||
|469870|Reaper's Onslaught|PASS||
|1229376|Single-Button Assistant|PASS||
|1233448|Dark Transformation|PASS||
|1234559|Desecrate|PASS||
|1238900|Grave Mastery|PASS||
|1241567|Clawing Shadows|PASS||
|1241705|Outnumber|PASS||
|1241858|Unholy Devotion|PASS||
|1242147|Summon Gargoyle|PASS||
|1242604|Scourging|PASS||
|1242608|Raise Abomination|PASS||
|1247378|Putrefy|PASS||
|1254552|Blightburst|PASS||
|1255830|Lesser Ghoul|PASS||
|1256813|Lord of the Dead|PASS||
|1256918|Unholy Death Knight|FAIL|a:aura:647; a:aura:649|
|1265547|Thrill of Blood|PASS||
|1265574|Transfusion|PASS||
|1265855|Echoing Fury|PASS||
|1265859|Frigid Resolve|PASS||
|1265932|Deathly Blows|PASS||
|1265949|Let Terror Reign|PASS||
|1265959|Ride or Die!|PASS||
|1265971|Unholy Armaments|FAIL|a:aura:382|
|1266818|Death Defiance|PASS||
|1266819|Death Notes|PASS||
|1267028|Blood Bond|PASS||
|1271974|Blightfall|PASS||
|1280658|Inevitable|PASS||
|1290864|Cycle of Death|PASS||

### Hunter / Beast Mastery (253)

|Spell|Name|Result|Observed causes|
|---:|---|---|---|
|136|Mend Pet|PASS||
|781|Disengage|PASS||
|883|Call Pet 1|FAIL|d:fixture:NotSupported:You do not have a pet to summon|
|982|Revive Pet|FAIL|d:fixture:no_dead_owned_pet|
|1462|Beast Lore|FAIL|d:fixture:ineligible_creature_type|
|1494|Track Beasts|PASS||
|1513|Scare Beast|FAIL|d:fixture:ineligible_creature_type|
|1515|Tame Beast|FAIL|d:fixture:ineligible_creature_type|
|1543|Flare|PASS||
|2641|Dismiss Pet|FAIL|d:fixture:NotSupported:You do not have a pet.|
|5116|Concussive Shot|PASS||
|5384|Feign Death|FAIL|a:aura:66|
|6991|Feed Pet|FAIL|a:effect:101|
|19574|Bestial Wrath|PASS||
|19577|Intimidation|PASS||
|19801|Tranquilizing Shot|PASS||
|19878|Track Demons|PASS||
|19879|Track Dragonkin|PASS||
|19880|Track Elementals|PASS||
|19882|Track Giants|PASS||
|19883|Track Humanoids|PASS||
|19884|Track Undead|PASS||
|19885|Track Hidden|FAIL|a:aura:151|
|34026|Kill Command|PASS||
|34477|Misdirection|FAIL|a:effect:130|
|53270|Exotic Beasts|FAIL|d:fixture:logger_external_consumer:146|
|53480|Roar of Sacrifice|PASS||
|56641|Steady Shot|PASS||
|61648|Aspect of the Chameleon|FAIL|a:aura:120|
|76657|Mastery: Master of Beasts|PASS||
|83242|Call Pet 2|FAIL|d:fixture:NotSupported:You do not have a pet to summon|
|83243|Call Pet 3|FAIL|d:fixture:NotSupported:You do not have a pet to summon|
|83244|Call Pet 4|FAIL|d:fixture:NotSupported:You do not have a pet to summon|
|83245|Call Pet 5|FAIL|d:fixture:NotSupported:You do not have a pet to summon|
|86538|Mail Specialization|FAIL|d:fixture:no_passive_aura|
|93321|Control Pet|PASS||
|93322|Control Pet|PASS||
|109215|Posthaste|PASS||
|109248|Binding Shot|FAIL|d:fixture:ground138_not_placed|
|109304|Exhilaration|PASS||
|115939|Beast Cleave|PASS||
|117526|Binding Shot|PASS||
|120679|Dire Beast|PASS||
|125050|Fetch|PASS||
|127933|Fireworks|PASS||
|137014|Hunter|PASS||
|137015|Beast Mastery Hunter|PASS||
|138430|Ancient Zandalari Knowledge|PASS||
|147362|Counter Shot|PASS||
|157443|Critical Strikes|PASS||
|162697|Stat Negation Aura - Agility DPS|PASS||
|175686|Stopping Power|FAIL|d:fixture:InvalidTarget:SPELL_FAILED_CASTER_AURASTATE|
|185358|Arcane Shot|PASS||
|186257|Aspect of the Cheetah|PASS||
|186265|Aspect of the Turtle|PASS||
|187650|Freezing Trap|PASS||
|187698|Tar Trap|PASS||
|191384|Aspect of the Beast|PASS||
|193455|Cobra Shot|PASS||
|193532|Scent of Blood|PASS||
|195645|Wing Clip|PASS||
|199483|Camouflage|PASS||
|199530|Stomp|PASS||
|199532|Killer Cobra|PASS||
|199921|Trailblazer|PASS||
|205154|Mecha-Bond Imprint Matrix|PASS||
|209997|Play Dead|PASS||
|210000|Wake Up|PASS||
|217200|Barbed Shot|PASS||
|226178|missing|FAIL|d:missing_store|
|229533|Track Mechanicals|PASS||
|231018|missing|FAIL|d:missing_store|
|231548|The Beast Within|PASS||
|242155|Hybrid Kinship|PASS||
|257284|Hunter's Mark|FAIL|a:aura:68|
|264735|Survival of the Fittest|PASS||
|266921|Born To Be Wild|PASS||
|267116|Animal Companion|FAIL|a:aura:493|
|269737|Alpha Predator|PASS||
|270581|Natural Mending|PASS||
|272651|Command Pet|PASS||
|273887|Killer Instinct|PASS||
|321014|Pack Tactics|PASS||
|321297|Eyes of the Beast|PASS||
|343242|Wilderness Medicine|PASS||
|343244|Disruptive Rounds|PASS||
|343247|Improved Traps|PASS||
|354831|Wailing Arrow|PASS||
|355589|Wailing Arrow|PASS||
|355596|Wailing Arrow|PASS||
|378002|Pathfinding|PASS||
|378004|Keen Eyesight|PASS||
|378207|Kill Cleave|PASS||
|378209|Training Expert|PASS||
|378244|Cobra Senses|PASS||
|378743|Dire Command|PASS||
|384799|Hunter's Avoidance|PASS||
|385539|Rejuvenating Wind|PASS||
|385810|Dire Frenzy|PASS||
|386870|Brutal Companion|PASS||
|388039|Lone Survivor|PASS||
|392053|Piercing Fangs|PASS||
|393933|War Orders|PASS||
|407412|Bloody Frenzy|PASS||
|424557|Savagery|PASS||
|424558|Master Handler|PASS||
|430709|Smoke Screen|PASS||
|430712|Dark Chains|PASS||
|450373|Don't Look Back|PASS||
|450379|Invigorating Pulse|PASS||
|459450|Padded Armor|PASS||
|459455|Scout's Instincts|PASS||
|459460|Tar-Coated Bindings|PASS||
|459488|Moment of Opportunity|PASS||
|459502|Serrated Tips|PASS||
|459506|Kindling Flare|PASS||
|459507|Territorial Instincts|PASS||
|459517|Emergency Salve|PASS||
|459527|Unnatural Causes|PASS||
|459534|Trigger Finger|PASS||
|459542|Specialized Arsenal|PASS||
|459546|No Hard Feelings|PASS||
|459550|Go for the Throat|PASS||
|459552|Laceration|PASS||
|459693|Thundering Hooves|PASS||
|459730|Huntmaster's Call|PASS||
|459983|Kodo Tranquilizer|PASS||
|459991|Devilsaur Tranquilizer|PASS||
|462079|Beast Mastery Hunter|PASS||
|462080|Hunter|FAIL|a:aura:320|
|466930|Black Arrow|PASS||
|466932|Black Arrow|PASS||
|466990|Withering Fire|PASS||
|467644|The Bell Tolls|PASS||
|467741|Shadow Dagger|PASS||
|467749|Bleak Arrows|PASS||
|467897|Ebon Bowstring|PASS||
|467902|Banshee's Mark|PASS||
|467911|Bleak Powder|PASS||
|468695|Snakeskin Quiver|PASS||
|468701|Serpentine Strikes|PASS||
|469638|Soul Drinker|PASS||
|469880|Barbed Scales|PASS||
|471876|Howl of the Pack Leader|PASS||
|472352|Dire Summons|PASS||
|472357|Better Together|PASS||
|472358|Pack Mentality|PASS||
|472476|Ursine Fury|PASS||
|472524|Sharpened Claws|PASS||
|472550|Fury of the Wyvern|PASS||
|472639|Hogstrider|PASS||
|472660|No Mercy|PASS||
|472707|Shell Cover|PASS||
|472719|Slicked Shoes|PASS||
|472729|Horsehair Tether|PASS||
|472741|Stampede!|PASS||
|474746|Solitary Companion|PASS||
|1217524|Dire Cleave|PASS||
|1229376|Single-Button Assistant|PASS||
|1232739|Wildspeaker|PASS||
|1235397|Umbral Reach|PASS||
|1244920|Track Pets|PASS||
|1253599|Sentinel|PASS||
|1253732|Lunar Storm|PASS||
|1253751|Stargazer|PASS||
|1253807|Open Fire|PASS||
|1253825|Moon's Blessing|PASS||
|1253830|Can't Miss, Won't Miss|PASS||
|1253831|Sanctified Armaments|PASS||
|1253846|Arcane Talons|PASS||
|1253852|Lunar Calling|PASS||
|1253887|Conditioning|PASS||
|1253892|Scout's Vigil|PASS||
|1256939|Beast Mastery Hunter|FAIL|a:aura:647; a:aura:648|
|1258402|Touch of Grass|PASS||
|1258404|Catlike Reflexes|PASS||
|1258407|Improved Aspect of the Cheetah|PASS||
|1258485|Improved Aspect of the Turtle|PASS||
|1258486|Improved Feign Death|PASS||
|1258509|Guttural Roar|PASS||
|1264289|Corpsecaller|PASS||
|1264290|Wailing Dead|PASS||
|1264291|Blighted Quiver|PASS||
|1264359|Wild Thrash|PASS||
|1264690|Pact of the Hollow|PASS||
|1264775|Sharpened Fangs|PASS||
|1264781|Lethal Barbs|PASS||
|1264792|Wyvern's Gaze|PASS||
|1264797|Hoof and Blade|PASS||
|1264902|Moonlight Chakram|PASS||
|1264903|Radiant Edge|PASS||
|1264904|Twilight Requiem|PASS||
|1264934|Frenzy|PASS||
|1265044|Jagged Wounds|PASS||
|1265051|Thrill of the Hunt|PASS||
|1265052|Razor Sharp|PASS||
|1266069|Stalk and Strike|PASS||
|1267003|Precision Strikes|PASS||
|1267218|Shell Wall|PASS||
|1268671|Cold Feet|PASS||
|1268705|Masterful Call|PASS||
|1268868|Improved Snaring|PASS||
|1268871|Combat Experience|PASS||
|1272094|Guardian's Hide|PASS||
|1272099|Bloodshed|PASS||
|1277565|Through the Eyes|PASS||

### Hunter / Marksmanship (254)

|Spell|Name|Result|Observed causes|
|---:|---|---|---|
|136|Mend Pet|PASS||
|781|Disengage|PASS||
|883|Call Pet 1|FAIL|d:fixture:NotSupported:You do not have a pet to summon|
|982|Revive Pet|FAIL|d:fixture:no_dead_owned_pet|
|1462|Beast Lore|FAIL|d:fixture:ineligible_creature_type|
|1494|Track Beasts|PASS||
|1513|Scare Beast|FAIL|d:fixture:ineligible_creature_type|
|1515|Tame Beast|FAIL|d:fixture:ineligible_creature_type|
|1543|Flare|PASS||
|2641|Dismiss Pet|FAIL|d:fixture:NotSupported:You do not have a pet.|
|5116|Concussive Shot|PASS||
|5384|Feign Death|FAIL|a:aura:66|
|6197|Eagle Eye|FAIL|a:effect:72|
|6991|Feed Pet|FAIL|a:effect:101|
|19434|Aimed Shot|PASS||
|19801|Tranquilizing Shot|PASS||
|19878|Track Demons|PASS||
|19879|Track Dragonkin|PASS||
|19880|Track Elementals|PASS||
|19882|Track Giants|PASS||
|19883|Track Humanoids|PASS||
|19884|Track Undead|PASS||
|19885|Track Hidden|FAIL|a:aura:151|
|34477|Misdirection|FAIL|a:effect:130|
|53351|Kill Shot|FAIL|d:fixture:healthy_execute_target|
|53480|Roar of Sacrifice|PASS||
|56641|Steady Shot|PASS||
|61648|Aspect of the Chameleon|FAIL|a:aura:120|
|83242|Call Pet 2|FAIL|d:fixture:NotSupported:You do not have a pet to summon|
|83243|Call Pet 3|FAIL|d:fixture:NotSupported:You do not have a pet to summon|
|83244|Call Pet 4|FAIL|d:fixture:NotSupported:You do not have a pet to summon|
|83245|Call Pet 5|FAIL|d:fixture:NotSupported:You do not have a pet to summon|
|86538|Mail Specialization|FAIL|d:fixture:no_passive_aura|
|93321|Control Pet|PASS||
|93322|Control Pet|PASS||
|109215|Posthaste|PASS||
|109248|Binding Shot|FAIL|d:fixture:ground138_not_placed|
|109304|Exhilaration|PASS||
|117526|Binding Shot|PASS||
|125050|Fetch|PASS||
|127933|Fireworks|PASS||
|137014|Hunter|PASS||
|137016|Marksmanship Hunter|PASS||
|138430|Ancient Zandalari Knowledge|PASS||
|147362|Counter Shot|PASS||
|157443|Critical Strikes|PASS||
|162697|Stat Negation Aura - Agility DPS|PASS||
|175686|Stopping Power|FAIL|d:fixture:InvalidTarget:SPELL_FAILED_CASTER_AURASTATE|
|185358|Arcane Shot|PASS||
|186257|Aspect of the Cheetah|PASS||
|186265|Aspect of the Turtle|PASS||
|187650|Freezing Trap|PASS||
|187698|Tar Trap|PASS||
|193468|Mastery: Sniper Training|PASS||
|194595|Lock and Load|PASS||
|195645|Wing Clip|PASS||
|199483|Camouflage|PASS||
|199921|Trailblazer|PASS||
|204089|Bullseye|PASS||
|205154|Mecha-Bond Imprint Matrix|PASS||
|209997|Play Dead|PASS||
|210000|Wake Up|PASS||
|212431|Explosive Shot|PASS||
|226178|missing|FAIL|d:missing_store|
|229533|Track Mechanicals|PASS||
|231018|missing|FAIL|d:missing_store|
|242155|Hybrid Kinship|PASS||
|257044|Rapid Fire|PASS||
|257284|Hunter's Mark|FAIL|a:aura:68|
|257620|Multi-Shot|PASS||
|257621|Trick Shots|PASS||
|260240|Precise Shots|PASS||
|260243|Volley|PASS||
|260309|Master Marksman|PASS||
|260404|Calling the Shots|PASS||
|264735|Survival of the Fittest|PASS||
|266921|Born To Be Wild|PASS||
|270581|Natural Mending|PASS||
|272651|Command Pet|PASS||
|288613|Trueshot|PASS||
|321460|Deadeye|PASS||
|343242|Wilderness Medicine|PASS||
|343244|Disruptive Rounds|PASS||
|343247|Improved Traps|PASS||
|343248|Deathblow|PASS||
|354831|Wailing Arrow|PASS||
|355589|Wailing Arrow|PASS||
|355596|Wailing Arrow|PASS||
|378002|Pathfinding|PASS||
|378004|Keen Eyesight|PASS||
|378767|Focused Aim|PASS||
|378913|Light Ammo|PASS||
|384799|Hunter's Avoidance|PASS||
|385539|Rejuvenating Wind|PASS||
|388039|Lone Survivor|PASS||
|389019|Bulletstorm|PASS||
|391559|Surging Shots|PASS||
|400456|Salvo|PASS||
|430709|Smoke Screen|PASS||
|430712|Dark Chains|PASS||
|450373|Don't Look Back|PASS||
|450379|Invigorating Pulse|PASS||
|459450|Padded Armor|PASS||
|459455|Scout's Instincts|PASS||
|459460|Tar-Coated Bindings|PASS||
|459488|Moment of Opportunity|PASS||
|459502|Serrated Tips|PASS||
|459506|Kindling Flare|PASS||
|459507|Territorial Instincts|PASS||
|459517|Emergency Salve|PASS||
|459527|Unnatural Causes|PASS||
|459534|Trigger Finger|PASS||
|459542|Specialized Arsenal|PASS||
|459546|No Hard Feelings|PASS||
|459783|Penetrating Shots|PASS||
|459794|Quick Draw|PASS||
|459802|Small Game Hunter|PASS||
|459983|Kodo Tranquilizer|PASS||
|459991|Devilsaur Tranquilizer|PASS||
|462080|Hunter|FAIL|a:aura:320|
|462081|Marksmanship Hunter|PASS||
|466867|Avian Specialization|PASS||
|466904|Harrier's Cry|PASS||
|466930|Black Arrow|PASS||
|466932|Black Arrow|PASS||
|466990|Withering Fire|PASS||
|467644|The Bell Tolls|PASS||
|467741|Shadow Dagger|PASS||
|467749|Bleak Arrows|PASS||
|467897|Ebon Bowstring|PASS||
|467902|Banshee's Mark|PASS||
|467911|Bleak Powder|PASS||
|469638|Soul Drinker|PASS||
|470943|Feathered Frenzy|PASS||
|470945|Aspect of the Hydra|PASS||
|471348|On Target|PASS||
|471350|Obsidian Arrowhead|PASS||
|471366|Tensile Bowstring|PASS||
|471369|Precision Detonation|PASS||
|471428|Accuracy By Volume|PASS||
|471876|Howl of the Pack Leader|PASS||
|472352|Dire Summons|PASS||
|472357|Better Together|PASS||
|472358|Pack Mentality|PASS||
|472476|Ursine Fury|PASS||
|472524|Sharpened Claws|PASS||
|472550|Fury of the Wyvern|PASS||
|472639|Hogstrider|PASS||
|472660|No Mercy|PASS||
|472707|Shell Cover|PASS||
|472719|Slicked Shoes|PASS||
|472729|Horsehair Tether|PASS||
|472741|Stampede!|PASS||
|473369|Eagle's Accuracy|PASS||
|473385|No Scope|PASS||
|473520|Unstable Trigger|PASS||
|473523|Windrunner Quiver|PASS||
|474421|Intimidation|PASS||
|474440|Cunning|PASS||
|474456|Tenacious|PASS||
|474738|Unerring Vision|PASS||
|1217788|Manhunter|PASS||
|1219616|Spotter's Mark|PASS||
|1223323|Unbreakable Bond|PASS||
|1229376|Single-Button Assistant|PASS||
|1232955|Unmatched Precision|PASS||
|1235397|Umbral Reach|PASS||
|1253599|Sentinel|PASS||
|1253732|Lunar Storm|PASS||
|1253751|Stargazer|PASS||
|1253807|Open Fire|PASS||
|1253825|Moon's Blessing|PASS||
|1253830|Can't Miss, Won't Miss|PASS||
|1253831|Sanctified Armaments|PASS||
|1253846|Arcane Talons|PASS||
|1253852|Lunar Calling|PASS||
|1253887|Conditioning|PASS||
|1253892|Scout's Vigil|PASS||
|1256940|Marksmanship Hunter|FAIL|a:aura:647; a:aura:648|
|1258402|Touch of Grass|PASS||
|1258404|Catlike Reflexes|PASS||
|1258407|Improved Aspect of the Cheetah|PASS||
|1258485|Improved Aspect of the Turtle|PASS||
|1258486|Improved Feign Death|PASS||
|1258509|Guttural Roar|PASS||
|1264289|Corpsecaller|PASS||
|1264290|Wailing Dead|PASS||
|1264291|Blighted Quiver|PASS||
|1264690|Pact of the Hollow|PASS||
|1264775|Sharpened Fangs|PASS||
|1264781|Lethal Barbs|PASS||
|1264792|Wyvern's Gaze|PASS||
|1264797|Hoof and Blade|PASS||
|1264902|Moonlight Chakram|PASS||
|1264903|Radiant Edge|PASS||
|1264904|Twilight Requiem|PASS||
|1266069|Stalk and Strike|PASS||
|1267003|Precision Strikes|PASS||
|1267218|Shell Wall|PASS||
|1268671|Cold Feet|PASS||
|1268705|Masterful Call|PASS||
|1268868|Improved Snaring|PASS||
|1268871|Combat Experience|PASS||
|1272094|Guardian's Hide|PASS||
|1277546|Focus Fire|PASS||
|1277565|Through the Eyes|PASS||
|1277572|Critical Precision|PASS||
|1301406|Tactical Reload|PASS||
|1319011|Blood Fletching|PASS||

### Hunter / Survival (255)

|Spell|Name|Result|Observed causes|
|---:|---|---|---|
|136|Mend Pet|PASS||
|781|Disengage|PASS||
|883|Call Pet 1|FAIL|d:fixture:NotSupported:You do not have a pet to summon|
|982|Revive Pet|FAIL|d:fixture:no_dead_owned_pet|
|1462|Beast Lore|FAIL|d:fixture:ineligible_creature_type|
|1494|Track Beasts|PASS||
|1513|Scare Beast|FAIL|d:fixture:ineligible_creature_type|
|1515|Tame Beast|FAIL|d:fixture:ineligible_creature_type|
|1543|Flare|PASS||
|2641|Dismiss Pet|FAIL|d:fixture:NotSupported:You do not have a pet.|
|5116|Concussive Shot|PASS||
|5384|Feign Death|FAIL|a:aura:66|
|6991|Feed Pet|FAIL|a:effect:101|
|19577|Intimidation|PASS||
|19801|Tranquilizing Shot|PASS||
|19878|Track Demons|PASS||
|19879|Track Dragonkin|PASS||
|19880|Track Elementals|PASS||
|19882|Track Giants|PASS||
|19883|Track Humanoids|PASS||
|19884|Track Undead|PASS||
|19885|Track Hidden|FAIL|a:aura:151|
|34477|Misdirection|FAIL|a:effect:130|
|53480|Roar of Sacrifice|PASS||
|56641|Steady Shot|PASS||
|61648|Aspect of the Chameleon|FAIL|a:aura:120|
|83242|Call Pet 2|FAIL|d:fixture:NotSupported:You do not have a pet to summon|
|83243|Call Pet 3|FAIL|d:fixture:NotSupported:You do not have a pet to summon|
|83244|Call Pet 4|FAIL|d:fixture:NotSupported:You do not have a pet to summon|
|83245|Call Pet 5|FAIL|d:fixture:NotSupported:You do not have a pet to summon|
|86538|Mail Specialization|FAIL|d:fixture:no_passive_aura|
|93321|Control Pet|PASS||
|93322|Control Pet|PASS||
|109215|Posthaste|PASS||
|109248|Binding Shot|FAIL|d:fixture:ground138_not_placed|
|109304|Exhilaration|PASS||
|117526|Binding Shot|PASS||
|125050|Fetch|PASS||
|127933|Fireworks|PASS||
|137014|Hunter|PASS||
|137017|Survival Hunter|PASS||
|138430|Ancient Zandalari Knowledge|PASS||
|157443|Critical Strikes|PASS||
|162697|Stat Negation Aura - Agility DPS|PASS||
|175686|Stopping Power|FAIL|d:fixture:InvalidTarget:SPELL_FAILED_CASTER_AURASTATE|
|185358|Arcane Shot|PASS||
|186257|Aspect of the Cheetah|PASS||
|186265|Aspect of the Turtle|PASS||
|186270|Raptor Strike|PASS||
|186289|Aspect of the Eagle|PASS||
|187650|Freezing Trap|PASS||
|187698|Tar Trap|PASS||
|187707|Muzzle|PASS||
|190925|Harpoon|PASS||
|193265|Hatchet Toss|PASS||
|195645|Wing Clip|PASS||
|199483|Camouflage|PASS||
|199921|Trailblazer|PASS||
|205154|Mecha-Bond Imprint Matrix|PASS||
|209997|Play Dead|PASS||
|210000|Wake Up|PASS||
|226178|missing|FAIL|d:missing_store|
|229533|Track Mechanicals|PASS||
|231018|missing|FAIL|d:missing_store|
|242155|Hybrid Kinship|PASS||
|257284|Hunter's Mark|FAIL|a:aura:68|
|259489|Kill Command|PASS||
|259495|Wildfire Bomb|PASS||
|260248|Bloodseeker|PASS||
|260285|Tip of the Spear|PASS||
|263135|Mastery: Spirit Bond|PASS||
|264332|Guerrilla Tactics|PASS||
|264735|Survival of the Fittest|PASS||
|266921|Born To Be Wild|PASS||
|270581|Natural Mending|PASS||
|272651|Command Pet|PASS||
|321290|Improved Wildfire Bomb|PASS||
|321297|Eyes of the Beast|PASS||
|343242|Wilderness Medicine|PASS||
|343244|Disruptive Rounds|PASS||
|343247|Improved Traps|PASS||
|354831|Wailing Arrow|PASS||
|355589|Wailing Arrow|PASS||
|355596|Wailing Arrow|PASS||
|378002|Pathfinding|PASS||
|378004|Keen Eyesight|PASS||
|378934|Lunge|PASS||
|378937|Explosives Expert|PASS||
|378950|Sweeping Spear|PASS||
|378955|Killer Companion|PASS||
|384799|Hunter's Avoidance|PASS||
|385539|Rejuvenating Wind|PASS||
|385737|Bloody Claws|PASS||
|388039|Lone Survivor|PASS||
|430709|Smoke Screen|PASS||
|430712|Dark Chains|PASS||
|450373|Don't Look Back|PASS||
|450379|Invigorating Pulse|PASS||
|459450|Padded Armor|PASS||
|459455|Scout's Instincts|PASS||
|459460|Tar-Coated Bindings|PASS||
|459488|Moment of Opportunity|PASS||
|459502|Serrated Tips|PASS||
|459506|Kindling Flare|PASS||
|459507|Territorial Instincts|PASS||
|459517|Emergency Salve|PASS||
|459527|Unnatural Causes|PASS||
|459534|Trigger Finger|PASS||
|459542|Specialized Arsenal|PASS||
|459546|No Hard Feelings|PASS||
|459843|Grenade Juggler|PASS||
|459939|Outland Venom|PASS||
|459964|Flanker's Advantage|PASS||
|459983|Kodo Tranquilizer|PASS||
|459991|Devilsaur Tranquilizer|PASS||
|460198|Wildfire Infusion|PASS||
|462036|Blackrock Munitions|PASS||
|462080|Hunter|FAIL|a:aura:320|
|462082|Survival Hunter|PASS||
|466930|Black Arrow|PASS||
|466932|Black Arrow|PASS||
|466990|Withering Fire|PASS||
|467644|The Bell Tolls|PASS||
|467741|Shadow Dagger|PASS||
|467749|Bleak Arrows|PASS||
|467897|Ebon Bowstring|PASS||
|467902|Banshee's Mark|PASS||
|467911|Bleak Powder|PASS||
|469638|Soul Drinker|PASS||
|471876|Howl of the Pack Leader|PASS||
|472352|Dire Summons|PASS||
|472357|Better Together|PASS||
|472358|Pack Mentality|PASS||
|472476|Ursine Fury|PASS||
|472524|Sharpened Claws|PASS||
|472550|Fury of the Wyvern|PASS||
|472639|Hogstrider|PASS||
|472660|No Mercy|PASS||
|472707|Shell Cover|PASS||
|472719|Slicked Shoes|PASS||
|472729|Horsehair Tether|PASS||
|472741|Stampede!|PASS||
|1229376|Single-Button Assistant|PASS||
|1235397|Umbral Reach|PASS||
|1244920|Track Pets|PASS||
|1250646|Takedown|PASS||
|1251717|Strike as One|PASS||
|1251718|Two Against Many|PASS||
|1251790|Savagery|PASS||
|1252708|Mongoose Fury|PASS||
|1252738|Wallop|PASS||
|1252931|Shellshock|PASS||
|1252943|Wildfire Imbuement|PASS||
|1253053|Razor Edge|PASS||
|1253137|Sic 'Em|PASS||
|1253172|Shrapnel Bomb|PASS||
|1253176|Flamebreak|PASS||
|1253599|Sentinel|PASS||
|1253732|Lunar Storm|PASS||
|1253751|Stargazer|PASS||
|1253807|Open Fire|PASS||
|1253825|Moon's Blessing|PASS||
|1253830|Can't Miss, Won't Miss|PASS||
|1253831|Sanctified Armaments|PASS||
|1253846|Arcane Talons|PASS||
|1253852|Lunar Calling|PASS||
|1253887|Conditioning|PASS||
|1253892|Scout's Vigil|PASS||
|1253945|Mongoose Rounds|PASS||
|1256938|Flanked|PASS||
|1256941|Survival Hunter|FAIL|a:aura:647; a:aura:648|
|1257011|Vulnerability|PASS||
|1258402|Touch of Grass|PASS||
|1258404|Catlike Reflexes|PASS||
|1258407|Improved Aspect of the Cheetah|PASS||
|1258485|Improved Aspect of the Turtle|PASS||
|1258486|Improved Feign Death|PASS||
|1258509|Guttural Roar|PASS||
|1261193|Boomstick|PASS||
|1261229|Wildfire Shells|PASS||
|1262409|Lethal Calibration|PASS||
|1262442|Bonding|PASS||
|1264289|Corpsecaller|PASS||
|1264290|Wailing Dead|PASS||
|1264291|Blighted Quiver|PASS||
|1264690|Pact of the Hollow|PASS||
|1264775|Sharpened Fangs|PASS||
|1264781|Lethal Barbs|PASS||
|1264792|Wyvern's Gaze|PASS||
|1264797|Hoof and Blade|PASS||
|1264902|Moonlight Chakram|PASS||
|1264903|Radiant Edge|PASS||
|1264904|Twilight Requiem|PASS||
|1266069|Stalk and Strike|PASS||
|1267003|Precision Strikes|PASS||
|1267218|Shell Wall|PASS||
|1268671|Cold Feet|PASS||
|1268705|Masterful Call|PASS||
|1268868|Improved Snaring|PASS||
|1268871|Combat Experience|PASS||
|1272094|Guardian's Hide|PASS||
|1272136|Quick Reload|PASS||
|1272139|Twin Fangs|PASS||
|1272154|Primal Surge|PASS||
|1277565|Through the Eyes|PASS||
|1277760|Dual Wield|PASS||
|1311963|Bombardier|PASS||

### Priest / Discipline (256)

|Spell|Name|Result|Observed causes|
|---:|---|---|---|
|17|Power Word: Shield|PASS||
|453|Mind Soothe|FAIL|a:aura:91|
|527|Purify|PASS||
|528|Dispel Magic|PASS||
|585|Smite|PASS||
|586|Fade|PASS||
|589|Shadow Word: Pain|PASS||
|605|Mind Control|FAIL|a:aura:2|
|1706|Levitate|PASS||
|2006|Resurrection|PASS||
|2061|Flash Heal|PASS||
|2096|Mind Vision|FAIL|d:fixture:self_with_exclude_caster|
|8092|Mind Blast|PASS||
|8122|Psychic Scream|PASS||
|9484|Shackle Horror|FAIL|d:fixture:ineligible_creature_type|
|10060|Power Infusion|PASS||
|19236|Desperate Prayer|PASS||
|21562|Power Word: Fortitude|PASS||
|32375|Mass Dispel|PASS||
|32379|Shadow Word: Death|PASS||
|33206|Pain Suppression|PASS||
|34433|Shadowfiend|PASS||
|45243|Focused Will|PASS||
|47515|Divine Aegis|PASS||
|47540|Penance|PASS||
|55676|Petrifying Scream|PASS||
|62618|Power Word: Barrier|PASS||
|64129|Body and Soul|PASS||
|73325|Leap of Faith|FAIL|d:fixture:self_with_exclude_caster|
|77489|Echo of Light|PASS||
|81749|Atonement|PASS||
|89745|Mysticism|FAIL|d:fixture:no_passive_aura|
|108942|Phantasm|PASS||
|108945|Angelic Bulwark|PASS||
|109186|Surge of Light|PASS||
|121536|Angelic Feather|PASS||
|132157|Holy Nova|PASS||
|137030|Priest|PASS||
|137032|Discipline Priest|PASS||
|161862|Ascension|FAIL|a:aura:210|
|162701|Stat Negation Aura - Intellect Healer|PASS||
|186440|Shadow Mend|PASS||
|193063|Protective Light|PASS||
|193134|Castigation|PASS||
|194509|Power Word: Radiance|PASS||
|196704|Psychic Voice|PASS||
|197045|Shield Discipline|PASS||
|197419|Penance|PASS||
|197862|Archangel|PASS||
|198068|Power of the Dark Side|PASS||
|200829|Plea|PASS||
|205364|Dominate Mind|FAIL|a:aura:6|
|212036|Mass Resurrection|PASS||
|215768|Blaze of Light|PASS||
|226178|missing|FAIL|d:missing_store|
|231018|missing|FAIL|d:missing_store|
|231682|Sanctuary|PASS||
|238063|Lenience|PASS||
|238100|Angel's Mercy|PASS||
|238135|Eternal Barrier|PASS||
|271534|Mastery: Grace|PASS||
|322115|Light's Promise|PASS||
|341167|Mental Agility|PASS||
|368275|Binding Heals|PASS||
|372969|Holy Ray|PASS||
|372972|Dark Indulgence|PASS||
|372991|Pain Transformation|PASS||
|373003|Revel in Darkness|PASS||
|373035|Protector of the Frail|PASS||
|373049|Indemnity|PASS||
|373054|Abyssal Reverie|PASS||
|373180|Harsh Discipline|PASS||
|373218|Sanguine Teachings|FAIL|a:aura:443|
|373223|Tithe Evasion|PASS||
|373427|Inescapable Torment|PASS||
|373446|Translucent Image|PASS||
|373450|Light's Inspiration|PASS||
|373456|Unwavering Will|PASS||
|373466|Twins of the Sun Priestess|PASS||
|390620|Move with Grace|PASS||
|390632|Improved Purify|PASS||
|390667|Spell Warding|PASS||
|390670|Improved Fade|PASS||
|390676|Inspiration|PASS||
|390684|Bright Pupil|PASS||
|390685|Enduring Luminescence|PASS||
|390686|Painful Punishment|PASS||
|390689|Pain and Suffering|PASS||
|390691|Borrowed Time|PASS||
|390693|Inner Focus|PASS||
|390786|Weal and Woe|PASS||
|390832|Expiation|PASS||
|390919|Sheer Terror|PASS||
|390972|Twist of Fate|PASS||
|393870|Improved Flash Heal|PASS||
|415416|Benevolence|PASS||
|421453|Ultimate Penitence|PASS||
|433905|Prophet's Will|PASS||
|440006|Purify Disease|PASS||
|440662|Preventive Measures|PASS||
|440669|Save the Day|PASS||
|440670|Divine Feathers|PASS||
|440671|Preemptive Care|PASS||
|440681|Waste No Time|PASS||
|440738|Foreseen Circumstances|PASS||
|440742|Twinsight|PASS||
|440766|Assured Safety|PASS||
|447444|Entropic Rift|PASS||
|448278|Inner Quietus|PASS||
|448403|Collapsing Void|FAIL|a:aura:647|
|449880|Voidheart|PASS||
|449912|Darkening Horizon|PASS||
|450138|Void Empowerment|PASS||
|450405|Void Blast|PASS||
|450612|Void Infusion|PASS||
|451018|Dark Energy|PASS||
|451204|No Escape|PASS||
|451234|Voidwraith|PASS||
|451311|Void Leech|PASS||
|451569|Embrace the Shadow|PASS||
|451840|Devour Matter|PASS||
|458718|Desperate Measures|PASS||
|459559|Phantom Reach|PASS||
|462097|Priest|FAIL|a:aura:320|
|462098|Discipline Priest|PASS||
|472361|Divine Procession|PASS||
|472433|Evangelism|PASS||
|472568|Encroaching Shadows|PASS||
|1229376|Single-Button Assistant|PASS||
|1235211|Shadow Tap|PASS||
|1240364|Death's Torment|PASS||
|1246549|Lightburst|PASS||
|1246799|Prompt Prognosis|PASS||
|1246802|Piety|PASS||
|1248423|Guiding Light|PASS||
|1249230|Focused Power|PASS||
|1249233|Everlasting Light|PASS||
|1250218|Purge the Wicked|PASS||
|1250293|Occultist|PASS||
|1250378|Spectral Illusion|PASS||
|1250691|Void Tendrils|PASS||
|1250806|False Autonomy|PASS||
|1250818|Spiritual Guidance|PASS||
|1250820|Strength of Resolve|PASS||
|1250835|Dark Enlightenment|PASS||
|1251029|Mindpierce|PASS||
|1251030|Painful Invocation|PASS||
|1251077|Twin Disciplines|PASS||
|1252217|Shadow Mend|PASS||
|1253724|Greater Smite|PASS||
|1256903|Discipline Priest|FAIL|a:aura:647|
|1266845|Quickened Pulse|PASS||
|1266856|Touch of the Void|PASS||
|1266883|Overwhelming Shadows|PASS||
|1272352|Words of the Wise|PASS||
|1272359|Prophet's Insight|PASS||
|1272363|Unfolding Vision|PASS||
|1280131|Searing Light|PASS||
|1280137|Mindbender|PASS||
|1298779|Grim Deliverance|PASS||

### Priest / Holy (257)

|Spell|Name|Result|Observed causes|
|---:|---|---|---|
|17|Power Word: Shield|PASS||
|453|Mind Soothe|FAIL|a:aura:91|
|527|Purify|PASS||
|528|Dispel Magic|PASS||
|585|Smite|PASS||
|586|Fade|PASS||
|589|Shadow Word: Pain|PASS||
|596|Prayer of Healing|PASS||
|605|Mind Control|FAIL|a:aura:2|
|1706|Levitate|PASS||
|2006|Resurrection|PASS||
|2050|Holy Word: Serenity|PASS||
|2061|Flash Heal|PASS||
|2096|Mind Vision|FAIL|d:fixture:self_with_exclude_caster|
|8122|Psychic Scream|PASS||
|9484|Shackle Horror|FAIL|d:fixture:ineligible_creature_type|
|10060|Power Infusion|PASS||
|14914|Holy Fire|PASS||
|19236|Desperate Prayer|PASS||
|20711|Spirit of Redemption|PASS||
|21562|Power Word: Fortitude|PASS||
|32375|Mass Dispel|PASS||
|32379|Shadow Word: Death|PASS||
|33076|Prayer of Mending|PASS||
|34861|Holy Word: Sanctify|PASS||
|45243|Focused Will|PASS||
|47788|Guardian Spirit|PASS||
|55676|Petrifying Scream|PASS||
|63733|Serendipity|PASS||
|64129|Body and Soul|PASS||
|64843|Divine Hymn|PASS||
|73325|Leap of Faith|FAIL|d:fixture:self_with_exclude_caster|
|77485|Mastery: Echo of Light|PASS||
|77489|Echo of Light|PASS||
|88625|Holy Word: Chastise|PASS||
|89745|Mysticism|FAIL|d:fixture:no_passive_aura|
|108942|Phantasm|PASS||
|108945|Angelic Bulwark|PASS||
|109186|Surge of Light|PASS||
|120517|Halo|PASS||
|121536|Angelic Feather|PASS||
|132157|Holy Nova|PASS||
|137030|Priest|PASS||
|137031|Holy Priest|PASS||
|161862|Ascension|FAIL|a:aura:210|
|162701|Stat Negation Aura - Intellect Healer|PASS||
|186440|Shadow Mend|PASS||
|193063|Protective Light|PASS||
|193155|Enlightenment|PASS||
|193157|Light's Resurgence|PASS||
|196704|Psychic Voice|PASS||
|196707|Afterlife|PASS||
|196985|Light of the Naaru|PASS||
|200128|Trail of Light|PASS||
|200183|Apotheosis|PASS||
|200199|Censure|PASS||
|200209|Guardian Angel|PASS||
|205364|Dominate Mind|FAIL|a:aura:6|
|212036|Mass Resurrection|PASS||
|226178|missing|FAIL|d:missing_store|
|231018|missing|FAIL|d:missing_store|
|235587|Miracle Worker|PASS||
|238100|Angel's Mercy|PASS||
|238136|Cosmic Ripple|PASS||
|341167|Mental Agility|PASS||
|341997|Renewed Faith|PASS||
|368275|Binding Heals|PASS||
|372307|Burning Vehemence|PASS||
|372370|Gales of Song|PASS||
|372616|Empyreal Blaze|PASS||
|373218|Sanguine Teachings|FAIL|a:aura:443|
|373223|Tithe Evasion|PASS||
|373446|Translucent Image|PASS||
|373450|Light's Inspiration|PASS||
|373456|Unwavering Will|PASS||
|373466|Twins of the Sun Priestess|PASS||
|390620|Move with Grace|PASS||
|390632|Improved Purify|PASS||
|390667|Spell Warding|PASS||
|390670|Improved Fade|PASS||
|390676|Inspiration|PASS||
|390919|Sheer Terror|PASS||
|390954|Crisis Management|PASS||
|390967|Prismatic Echoes|PASS||
|390972|Twist of Fate|PASS||
|390977|Prayers of the Virtuous|PASS||
|390992|Lightweaver|PASS||
|390994|Voice of Harmony|PASS||
|391124|Restitution|PASS||
|391186|Say Your Prayers|PASS||
|391209|Prayerful Litany|PASS||
|391233|Divine Service|PASS||
|391381|Desperate Times|FAIL|a:aura:359|
|392988|Divine Image|PASS||
|393870|Improved Flash Heal|PASS||
|414553|Epiphany|PASS||
|415416|Benevolence|PASS||
|419110|Seraphic Crescendo|PASS||
|433905|Prophet's Will|PASS||
|440006|Purify Disease|PASS||
|440662|Preventive Measures|PASS||
|440669|Save the Day|PASS||
|440670|Divine Feathers|PASS||
|440671|Preemptive Care|PASS||
|440681|Waste No Time|PASS||
|440738|Foreseen Circumstances|PASS||
|440742|Twinsight|PASS||
|440766|Assured Safety|PASS||
|449806|Divine Halo|PASS||
|449874|Energy Compression|PASS||
|453109|Power Surge|PASS||
|453726|Word of Supremacy|PASS||
|453729|Heightened Alteration|PASS||
|453783|Manifested Power|PASS||
|453799|Empowered Surges|PASS||
|453828|Energy Cycle|PASS||
|453845|Resonant Energy|PASS||
|453852|Shock Pulse|PASS||
|453917|Perfected Form|PASS||
|453918|Incessant Screams|PASS||
|454001|Sustained Potency|PASS||
|458718|Desperate Measures|PASS||
|459559|Phantom Reach|PASS||
|462097|Priest|FAIL|a:aura:320|
|462099|Holy Priest|PASS||
|471504|Lasting Words|PASS||
|471668|Light in the Darkness|PASS||
|1215241|Divinity|PASS||
|1215245|Eternal Sanctity|PASS||
|1215265|Dispersing Light|PASS||
|1215275|Holy Celerity|PASS||
|1229376|Single-Button Assistant|PASS||
|1238052|Holy Priest|FAIL|a:aura:647; a:aura:649|
|1246517|Ultimate Serenity|PASS||
|1246549|Lightburst|PASS||
|1246799|Prompt Prognosis|PASS||
|1246802|Piety|PASS||
|1247131|Efficient Prayers|PASS||
|1247166|Healing Focus|PASS||
|1247178|Spiritwell|PASS||
|1248418|Cosmic Wave|PASS||
|1248423|Guiding Light|PASS||
|1249230|Focused Power|PASS||
|1249233|Everlasting Light|PASS||
|1250378|Spectral Illusion|PASS||
|1250562|Uplifting Words|PASS||
|1250565|Angelic Touch|PASS||
|1250684|Radiant Plea|PASS||
|1250691|Void Tendrils|PASS||
|1250806|False Autonomy|PASS||
|1250818|Spiritual Guidance|PASS||
|1250820|Strength of Resolve|PASS||
|1250835|Dark Enlightenment|PASS||
|1251029|Mindpierce|PASS||
|1251030|Painful Invocation|PASS||
|1251077|Twin Disciplines|PASS||
|1272308|Energy Conservation|PASS||
|1272320|Focused Outburst|PASS||
|1272326|Realized Potential|PASS||
|1272352|Words of the Wise|PASS||
|1272359|Prophet's Insight|PASS||
|1272363|Unfolding Vision|PASS||

### Priest / Shadow (258)

|Spell|Name|Result|Observed causes|
|---:|---|---|---|
|17|Power Word: Shield|PASS||
|453|Mind Soothe|FAIL|a:aura:91|
|528|Dispel Magic|PASS||
|585|Smite|PASS||
|586|Fade|PASS||
|589|Shadow Word: Pain|PASS||
|605|Mind Control|FAIL|a:aura:2|
|1706|Levitate|PASS||
|2006|Resurrection|PASS||
|2061|Flash Heal|PASS||
|2096|Mind Vision|FAIL|d:fixture:self_with_exclude_caster|
|8092|Mind Blast|PASS||
|8122|Psychic Scream|PASS||
|9484|Shackle Horror|FAIL|d:fixture:ineligible_creature_type|
|10060|Power Infusion|PASS||
|15286|Vampiric Embrace|PASS||
|15407|Mind Flay|PASS||
|15487|Silence|PASS||
|19236|Desperate Prayer|PASS||
|21562|Power Word: Fortitude|PASS||
|32375|Mass Dispel|PASS||
|32379|Shadow Word: Death|PASS||
|34433|Shadowfiend|PASS||
|34914|Vampiric Touch|PASS||
|45243|Focused Will|PASS||
|47585|Dispersion|PASS||
|55676|Petrifying Scream|PASS||
|64129|Body and Soul|PASS||
|73325|Leap of Faith|FAIL|d:fixture:self_with_exclude_caster|
|73510|Shadeburst|PASS||
|77489|Echo of Light|PASS||
|89745|Mysticism|FAIL|d:fixture:no_passive_aura|
|108942|Phantasm|PASS||
|108945|Angelic Bulwark|PASS||
|109186|Surge of Light|PASS||
|120644|Halo|PASS||
|121536|Angelic Feather|PASS||
|132157|Holy Nova|PASS||
|137030|Priest|PASS||
|137033|Shadow Priest|PASS||
|155271|Auspicious Spirits|PASS||
|161862|Ascension|FAIL|a:aura:210|
|162699|Stat Negation Aura - Intellect DPS|PASS||
|186440|Shadow Mend|PASS||
|193063|Protective Light|PASS||
|194248|missing|FAIL|d:missing_store|
|196704|Psychic Voice|PASS||
|199484|Psychic Link|PASS||
|205364|Dominate Mind|FAIL|a:aura:6|
|213634|Purify Disease|PASS||
|226178|missing|FAIL|d:missing_store|
|228260|Voidform|PASS||
|231018|missing|FAIL|d:missing_store|
|232698|Shadowform|PASS||
|238100|Angel's Mercy|PASS||
|238558|Misery|PASS||
|263165|Void Torrent|FAIL|a:aura:470|
|280752|Hallucinations|PASS||
|288733|Intangibility|PASS||
|321291|Death and Madness|PASS||
|335467|Shadow Word: Madness|FAIL|d:fixture:OnlyShapeshift:|
|341167|Mental Agility|PASS||
|341240|Improved Voidform|PASS||
|341491|Shadowy Apparitions|PASS||
|343690|Mastery: Shadow Weaving|PASS||
|368275|Binding Heals|PASS||
|373202|Mind Devourer|PASS||
|373212|Insidious Ire|PASS||
|373218|Sanguine Teachings|FAIL|a:aura:443|
|373223|Tithe Evasion|PASS||
|373273|Idol of Yogg-Saron|PASS||
|373280|Idol of N'Zoth|PASS||
|373310|Idol of Y'Shaarj|PASS||
|373427|Inescapable Torment|PASS||
|373446|Translucent Image|PASS||
|373450|Light's Inspiration|PASS||
|373456|Unwavering Will|PASS||
|373466|Twins of the Sun Priestess|PASS||
|375767|Screams of the Void|PASS||
|375888|Shadowy Insight|PASS||
|375994|Mental Decay|PASS||
|377065|Mental Fortitude|PASS||
|377349|Idol of C'Thun|PASS||
|390620|Move with Grace|PASS||
|390667|Spell Warding|PASS||
|390670|Improved Fade|PASS||
|390676|Inspiration|PASS||
|390919|Sheer Terror|PASS||
|390972|Twist of Fate|PASS||
|391090|Shattered Psyche|PASS||
|391099|Dark Evangelism|PASS||
|391151|Mastermind|PASS||
|391228|Maddening Touch|PASS||
|391284|Tormented Spirits|PASS||
|391399|Surge of Insanity|PASS||
|392507|Deathspeaker|PASS||
|393870|Improved Flash Heal|PASS||
|397527|Insanity|PASS||
|406788|Thought Harvester|PASS||
|407430|Voidtouched|PASS||
|407469|Spectral Horrors|PASS||
|407470|Mind's Eye|PASS||
|409044|Distorted Reality|PASS||
|415416|Benevolence|PASS||
|448278|Inner Quietus|PASS||
|448403|Collapsing Void|FAIL|a:aura:647|
|449806|Divine Halo|PASS||
|449874|Energy Compression|PASS||
|449880|Voidheart|PASS||
|449912|Darkening Horizon|PASS||
|450138|Void Empowerment|PASS||
|450405|Void Blast|PASS||
|450612|Void Infusion|PASS||
|451018|Dark Energy|PASS||
|451204|No Escape|PASS||
|451234|Voidwraith|PASS||
|451311|Void Leech|PASS||
|451569|Embrace the Shadow|PASS||
|451840|Devour Matter|PASS||
|453109|Power Surge|PASS||
|453726|Word of Supremacy|PASS||
|453729|Heightened Alteration|PASS||
|453783|Manifested Power|PASS||
|453799|Empowered Surges|PASS||
|453828|Energy Cycle|PASS||
|453845|Resonant Energy|PASS||
|453852|Shock Pulse|PASS||
|453917|Perfected Form|PASS||
|453918|Incessant Screams|PASS||
|454001|Sustained Potency|PASS||
|459559|Phantom Reach|PASS||
|462097|Priest|FAIL|a:aura:320|
|462101|Shadow Priest|PASS||
|1227280|Tentacle Slam|PASS||
|1228516|Subservient Shadows|PASS||
|1229376|Single-Button Assistant|PASS||
|1230339|Mindbender|PASS||
|1231346|Ancient Madness|PASS||
|1240364|Death's Torment|PASS||
|1240388|Dark Thoughts|PASS||
|1240394|Madness Weaving|PASS||
|1242666|Descending Darkness|PASS||
|1242862|Instilled Doubt|PASS||
|1246549|Lightburst|PASS||
|1247178|Spiritwell|PASS||
|1249230|Focused Power|PASS||
|1249233|Everlasting Light|PASS||
|1250378|Spectral Illusion|PASS||
|1250492|Tormenting Whispers|PASS||
|1250691|Void Tendrils|PASS||
|1250806|False Autonomy|PASS||
|1250818|Spiritual Guidance|PASS||
|1250820|Strength of Resolve|PASS||
|1250835|Dark Enlightenment|PASS||
|1251029|Mindpierce|PASS||
|1251030|Painful Invocation|PASS||
|1251077|Twin Disciplines|PASS||
|1256905|Shadow Priest|FAIL|a:aura:647; a:aura:649|
|1266845|Quickened Pulse|PASS||
|1266856|Touch of the Void|PASS||
|1266883|Overwhelming Shadows|PASS||
|1272308|Energy Conservation|PASS||
|1272320|Focused Outburst|PASS||
|1272326|Realized Potential|PASS||
|1279350|Invoked Nightmare|PASS||
|1279353|Maddening Tentacles|PASS||
|1279354|Crushing Void|PASS||
|1279355|Haunting Shadows|PASS||

### Rogue / Assassination (259)

|Spell|Name|Result|Observed causes|
|---:|---|---|---|
|408|Kidney Shot|PASS||
|703|Garrote|PASS||
|921|Pick Pocket|FAIL|d:fixture:OnlyShapeshift:|
|1329|Mutilate|PASS||
|1725|Distract|PASS||
|1752|Sinister Strike|PASS||
|1766|Kick|PASS||
|1776|Gouge|PASS||
|1784|Stealth|FAIL|a:aura:443|
|1804|Pick Lock|FAIL|d:fixture:no_lock_item_or_object|
|1833|Cheap Shot|FAIL|d:fixture:OnlyShapeshift:|
|1856|Vanish|FAIL|a:effect:93|
|1860|Safe Fall|PASS||
|1943|Rupture|PASS||
|1966|Feint|PASS||
|2094|Blind|PASS||
|2818|Deadly Poison|PASS||
|2823|Deadly Poison|PASS||
|2836|Detect Traps|PASS||
|2983|Sprint|PASS||
|3408|Crippling Poison|PASS||
|5277|Evasion|PASS||
|5374|Mutilate|PASS||
|5761|Numbing Poison|PASS||
|5938|Shiv|PASS||
|6770|Sap|FAIL|d:fixture:OnlyShapeshift:|
|8676|Ambush|FAIL|d:fixture:OnlyShapeshift:|
|8679|Wound Poison|PASS||
|14190|Seal Fate|PASS||
|14983|Vigor|PASS||
|27576|Mutilate|PASS||
|31224|Cloak of Shadows|PASS||
|31230|Cheat Death|PASS||
|32645|Envenom|PASS||
|51667|Cut to the Chase|PASS||
|51723|Fan of Knives|PASS||
|56814|Detection|PASS||
|57934|Tricks of the Trade|FAIL|d:fixture:self_with_exclude_caster|
|76803|Mastery: Potent Assassin|PASS||
|79008|Elusiveness|PASS||
|79134|Venomous Wounds|PASS||
|82245|Parry|PASS||
|86092|Leather Specialization|FAIL|d:fixture:no_passive_aura|
|86392|Main Gauche|PASS||
|108208|Subterfuge|PASS||
|113780|Deadly Poison|PASS||
|114018|Shroud of Concealment|FAIL|d:fixture:OnlyShapeshift:|
|137034|Rogue|PASS||
|137037|Assassination Rogue|PASS||
|157442|Critical Strikes|PASS||
|162697|Stat Negation Aura - Agility DPS|PASS||
|185311|Crimson Vial|PASS||
|185565|Poisoned Knife|PASS||
|193531|Deeper Stratagem|PASS||
|193539|Alacrity|PASS||
|193546|Iron Stomach|PASS||
|196819|Eviscerate|PASS||
|196861|Iron Wire|PASS||
|200733|Airborne Irritant|PASS||
|210108|Detection|PASS||
|226178|missing|FAIL|d:missing_store|
|231018|missing|FAIL|d:missing_store|
|231691|Improved Sprint|PASS||
|231719|Deadened Nerves|PASS||
|255544|Poison Bomb|PASS||
|280716|Leeching Poison|PASS||
|315496|Slice and Dice|PASS||
|315584|Instant Poison|PASS||
|319066|Improved Wound Poison|PASS||
|319473|Mastery: Potent Assassin|PASS||
|328085|Blindside|PASS||
|360194|Deathmark|PASS||
|378427|Nimble Fingers|PASS||
|378436|Master Poisoner|PASS||
|378807|Shadowrunner|PASS||
|378813|Fleet Footed|PASS||
|378996|Recuperator|PASS||
|379005|Blackjack|PASS||
|381542|Deadly Precision|PASS||
|381543|Virulent Poisons|PASS||
|381620|Improved Ambush|PASS||
|381621|Tight Spender|PASS||
|381624|Improved Poisons|PASS||
|381626|Bloody Mess|PASS||
|381627|Internal Bleeding|PASS||
|381629|Thrown Precision|PASS||
|381630|Intent to Kill|PASS||
|381631|Flying Daggers|PASS||
|381632|Improved Garrote|PASS||
|381637|Atrophic Poison|PASS||
|381640|Lethal Dose|PASS||
|381652|Systemic Failure|PASS||
|381664|Amplifying Poison|PASS||
|381673|Doomblade|PASS||
|381797|Dashing Scoundrel|PASS||
|381798|Zoldyck Recipe|PASS||
|381799|Scent of Blood|PASS||
|381801|Dragon-Tempered Blades|PASS||
|381988|Swift Slasher|PASS||
|382238|Lethality|PASS||
|382245|Cold Blooded Killer|PASS||
|382513|Without a Trace|PASS||
|385478|Shrouded Suffocation|PASS||
|385627|Kingsbane|PASS||
|392384|Fatal Concoction|PASS||
|393970|Soothing Darkness|PASS||
|394931|Thrill Seeking|PASS||
|394932|Shadowstep|PASS||
|400804|Unbreakable Stride|PASS||
|421975|Caustic Spatter|PASS||
|423054|Path of Blood|PASS||
|423136|Sudden Demise|PASS||
|423647|Graceful Guile|PASS||
|423662|Stillshroud|PASS||
|423683|Featherfoot|PASS||
|423701|Superior Mixture|PASS||
|441247|Smoke|PASS||
|441250|Mirrors|PASS||
|441263|Devious Distractions|PASS||
|441273|Surprising Strikes|PASS||
|441274|Disorienting Strikes|PASS||
|441321|Flawless Form|PASS||
|441346|Thousand Cuts|PASS||
|441359|Flickerstrike|PASS||
|441367|Nimble Flurry|PASS||
|441398|No Scruples|PASS||
|441403|So Tricky|PASS||
|441415|Don't Be Suspicious|PASS||
|441423|Coup de Grace|PASS||
|441429|Cloud Cover|PASS||
|452536|Hand of Fate|PASS||
|453428|Mean Streak|PASS||
|453457|Edge Case|PASS||
|454286|Tempted Fate|PASS||
|454419|Deal Fate|PASS||
|454432|Inexorable March|PASS||
|454433|Death's Arrival|PASS||
|454435|Destiny Defined|PASS||
|455072|Rapid Injection|PASS||
|457022|Ethereal Cloak|PASS||
|457034|Bait and Switch|PASS||
|457052|Deathstalker's Mark|PASS||
|457054|Hunt Them Down|PASS||
|457055|Singular Focus|PASS||
|457056|Lingering Darkness|PASS||
|457057|Shadewalker|PASS||
|457058|Darkest Night|PASS||
|457062|Symbolic Victory|PASS||
|457063|Shroud of Night|PASS||
|457067|Momentum of Despair|PASS||
|457068|Follow the Blood|PASS||
|457512|Sanguine Stratagem|PASS||
|462102|Assassination Rogue|PASS||
|462103|Rogue|PASS||
|469779|Thistle Tea|PASS||
|470347|Supercharger|PASS||
|470668|Forced Induction|PASS||
|470669|Echoing Reprimand|PASS||
|1229376|Single-Button Assistant|PASS||
|1247227|Crimson Tempest|PASS||
|1247993|Motivated Murderer|PASS||
|1248774|Unshakeable Drive|PASS||
|1248785|Corrupt the Blood|PASS||
|1248793|Clear the Witnesses|PASS||
|1248970|Lucky Coin|PASS||
|1249190|Overflowing Purse|PASS||
|1249194|Delivered Doom|PASS||
|1249201|Chosen's Revelry|PASS||
|1249204|Rush to the Inevitable|PASS||
|1249215|Fate Intertwined|PASS||
|1249802|Razor Wire|PASS||
|1249809|Finish the Job|PASS||
|1250036|Inspiring Strike|PASS||
|1250141|Secondary Poisoning|PASS||
|1250318|Poisoner's Drive|PASS||
|1250325|Regicide's Reward|PASS||
|1250358|Avulsion|PASS||
|1250359|Canny Strikes|PASS||
|1256950|Assassination Rogue|FAIL|a:aura:647|
|1267182|Toxic Stiletto|PASS||
|1267210|Quick Fingers|PASS||
|1267216|Deep Cuts|PASS||
|1267220|Danger Sense|PASS||
|1272989|Precise Killer|PASS||
|1273017|Quietus Celeris|PASS||
|1273035|Mass Casualty|PASS||
|1276626|Hoodwink|PASS||
|1276630|Flashing Steel|PASS||
|1276679|Clever Combatant|PASS||
|1276809|Ravenholdt Mint|PASS||
|1276816|Controlled Chaos|PASS||
|1277030|Sometimes Lucky|PASS||
|1292996|Negotiable Contract|PASS||
|1293135|Sanguine Vial|PASS||
|1298812|Unstable Toxin|PASS||
|1298826|Thistle Tea|PASS||

### Rogue / Outlaw (260)

|Spell|Name|Result|Observed causes|
|---:|---|---|---|
|408|Kidney Shot|PASS||
|921|Pick Pocket|FAIL|d:fixture:OnlyShapeshift:|
|1725|Distract|PASS||
|1752|Sinister Strike|PASS||
|1766|Kick|PASS||
|1776|Gouge|PASS||
|1784|Stealth|FAIL|a:aura:443|
|1804|Pick Lock|FAIL|d:fixture:no_lock_item_or_object|
|1833|Cheap Shot|FAIL|d:fixture:OnlyShapeshift:|
|1856|Vanish|FAIL|a:effect:93|
|1860|Safe Fall|PASS||
|1966|Feint|PASS||
|2094|Blind|PASS||
|2098|Dispatch|PASS||
|2818|Deadly Poison|PASS||
|2836|Detect Traps|PASS||
|2983|Sprint|PASS||
|3408|Crippling Poison|PASS||
|5277|Evasion|PASS||
|5374|Mutilate|PASS||
|5761|Numbing Poison|PASS||
|5938|Shiv|PASS||
|6770|Sap|FAIL|d:fixture:OnlyShapeshift:|
|8676|Ambush|FAIL|d:fixture:OnlyShapeshift:|
|8679|Wound Poison|PASS||
|13750|Adrenaline Rush|PASS||
|13877|Blade Flurry|PASS||
|14161|Ruthlessness|PASS||
|14983|Vigor|PASS||
|27576|Mutilate|PASS||
|31224|Cloak of Shadows|PASS||
|31230|Cheat Death|PASS||
|35551|Fatal Flourish|PASS||
|51667|Cut to the Chase|PASS||
|51690|Killing Spree|PASS||
|56814|Detection|PASS||
|57934|Tricks of the Trade|FAIL|d:fixture:self_with_exclude_caster|
|61329|Combat Potency|PASS||
|76806|Mastery: Main Gauche|PASS||
|79008|Elusiveness|PASS||
|79096|Restless Blades|PASS||
|82245|Parry|PASS||
|86092|Leather Specialization|FAIL|d:fixture:no_passive_aura|
|86392|Main Gauche|PASS||
|108208|Subterfuge|PASS||
|113780|Deadly Poison|PASS||
|114018|Shroud of Concealment|FAIL|d:fixture:OnlyShapeshift:|
|137034|Rogue|PASS||
|137036|Outlaw Rogue|PASS||
|157442|Critical Strikes|PASS||
|162697|Stat Negation Aura - Agility DPS|PASS||
|185311|Crimson Vial|PASS||
|185763|Pistol Shot|PASS||
|193315|Sinister Strike|PASS||
|193531|Deeper Stratagem|PASS||
|193539|Alacrity|PASS||
|193546|Iron Stomach|PASS||
|195457|Grappling Hook|PASS||
|196819|Eviscerate|PASS||
|196922|Hit and Run|PASS||
|196938|Quick Draw|PASS||
|199736|Find Treasure|FAIL|a:aura:45|
|200733|Airborne Irritant|PASS||
|210108|Detection|PASS||
|226178|missing|FAIL|d:missing_store|
|231018|missing|FAIL|d:missing_store|
|231691|Improved Sprint|PASS||
|231719|Deadened Nerves|PASS||
|235484|Improved Between the Eyes|PASS||
|256165|Blinding Powder|PASS||
|256170|Loaded Dice|PASS||
|256188|Retractable Hook|PASS||
|271877|Blade Rush|PASS||
|272026|Dancing Steel|PASS||
|279876|Opportunity|PASS||
|280716|Leeching Poison|PASS||
|315341|Between the Eyes|PASS||
|315496|Slice and Dice|PASS||
|315584|Instant Poison|PASS||
|319066|Improved Wound Poison|PASS||
|378427|Nimble Fingers|PASS||
|378436|Master Poisoner|PASS||
|378807|Shadowrunner|PASS||
|378813|Fleet Footed|PASS||
|378996|Recuperator|PASS||
|379005|Blackjack|PASS||
|381542|Deadly Precision|PASS||
|381543|Virulent Poisons|PASS||
|381619|Thief's Versatility|PASS||
|381620|Improved Ambush|PASS||
|381621|Tight Spender|PASS||
|381637|Atrophic Poison|PASS||
|381828|Ace Up Your Sleeve|PASS||
|381839|Sleight of Hand|PASS||
|381845|Audacity|PASS||
|381846|Fan the Hammer|PASS||
|381877|Combat Stamina|PASS||
|381878|Deft Maneuvers|PASS||
|381885|Heavy Hitter|PASS||
|381988|Swift Slasher|PASS||
|381989|Keep It Rolling|PASS||
|381990|Summarily Dispatched|PASS||
|382238|Lethality|PASS||
|382245|Cold Blooded Killer|PASS||
|382513|Without a Trace|PASS||
|383281|Hidden Opportunity|PASS||
|393970|Soothing Darkness|PASS||
|394321|Devious Stratagem|PASS||
|394931|Thrill Seeking|PASS||
|395422|Improved Adrenaline Rush|PASS||
|400804|Unbreakable Stride|PASS||
|423647|Graceful Guile|PASS||
|423662|Stillshroud|PASS||
|423683|Featherfoot|PASS||
|423701|Superior Mixture|PASS||
|428377|Precision Shot|PASS||
|441146|Unseen Blade|PASS||
|441247|Smoke|PASS||
|441250|Mirrors|PASS||
|441263|Devious Distractions|PASS||
|441273|Surprising Strikes|PASS||
|441274|Disorienting Strikes|PASS||
|441321|Flawless Form|PASS||
|441346|Thousand Cuts|PASS||
|441359|Flickerstrike|PASS||
|441367|Nimble Flurry|PASS||
|441398|No Scruples|PASS||
|441403|So Tricky|PASS||
|441415|Don't Be Suspicious|PASS||
|441423|Coup de Grace|PASS||
|441429|Cloud Cover|PASS||
|452536|Hand of Fate|PASS||
|453428|Mean Streak|PASS||
|453457|Edge Case|PASS||
|454286|Tempted Fate|PASS||
|454419|Deal Fate|PASS||
|454432|Inexorable March|PASS||
|454433|Death's Arrival|PASS||
|454435|Destiny Defined|PASS||
|455143|Acrobatic Strikes|PASS||
|457022|Ethereal Cloak|PASS||
|457034|Bait and Switch|PASS||
|457052|Deathstalker's Mark|PASS||
|457054|Hunt Them Down|PASS||
|457055|Singular Focus|PASS||
|457056|Lingering Darkness|PASS||
|457057|Shadewalker|PASS||
|457058|Darkest Night|PASS||
|457062|Symbolic Victory|PASS||
|457063|Shroud of Night|PASS||
|457067|Momentum of Despair|PASS||
|457068|Follow the Blood|PASS||
|462103|Rogue|PASS||
|462104|Outlaw Rogue|PASS||
|469779|Thistle Tea|PASS||
|470347|Supercharger|PASS||
|470668|Forced Induction|PASS||
|470669|Echoing Reprimand|PASS||
|1214909|Roll the Bones|PASS||
|1229376|Single-Button Assistant|PASS||
|1248774|Unshakeable Drive|PASS||
|1248785|Corrupt the Blood|PASS||
|1248793|Clear the Witnesses|PASS||
|1248970|Lucky Coin|PASS||
|1249190|Overflowing Purse|PASS||
|1249194|Delivered Doom|PASS||
|1249201|Chosen's Revelry|PASS||
|1249204|Rush to the Inevitable|PASS||
|1249215|Fate Intertwined|PASS||
|1256630|Menacing Rush|PASS||
|1256947|Outlaw Rogue|FAIL|a:aura:647|
|1259457|Find an Opening|PASS||
|1259465|Heightened Rush|PASS||
|1259469|Grand Melee|PASS||
|1259480|Fast Action|PASS||
|1259481|Dragon-Bone Dice|PASS||
|1259485|Zero In|PASS||
|1259492|Flickering Steel|PASS||
|1259498|Expert Duelist|PASS||
|1259499|Crescendo of Violence|PASS||
|1259612|Deadly Pursuit|PASS||
|1267182|Toxic Stiletto|PASS||
|1267210|Quick Fingers|PASS||
|1267216|Deep Cuts|PASS||
|1267220|Danger Sense|PASS||
|1272989|Precise Killer|PASS||
|1273017|Quietus Celeris|PASS||
|1273035|Mass Casualty|PASS||
|1276626|Hoodwink|PASS||
|1276630|Flashing Steel|PASS||
|1276679|Clever Combatant|PASS||
|1276809|Ravenholdt Mint|PASS||
|1276816|Controlled Chaos|PASS||
|1277030|Sometimes Lucky|PASS||
|1277933|Preparation|PASS||
|1293135|Sanguine Vial|PASS||
|1298826|Thistle Tea|PASS||

### Rogue / Subtlety (261)

|Spell|Name|Result|Observed causes|
|---:|---|---|---|
|53|Backstab|PASS||
|408|Kidney Shot|PASS||
|921|Pick Pocket|FAIL|d:fixture:OnlyShapeshift:|
|1725|Distract|PASS||
|1752|Sinister Strike|PASS||
|1766|Kick|PASS||
|1776|Gouge|PASS||
|1784|Stealth|FAIL|a:aura:443|
|1804|Pick Lock|FAIL|d:fixture:no_lock_item_or_object|
|1833|Cheap Shot|FAIL|d:fixture:OnlyShapeshift:|
|1856|Vanish|FAIL|a:effect:93|
|1860|Safe Fall|PASS||
|1966|Feint|PASS||
|2094|Blind|PASS||
|2818|Deadly Poison|PASS||
|2836|Detect Traps|PASS||
|2983|Sprint|PASS||
|3408|Crippling Poison|PASS||
|5277|Evasion|PASS||
|5374|Mutilate|PASS||
|5761|Numbing Poison|PASS||
|5938|Shiv|PASS||
|6770|Sap|FAIL|d:fixture:OnlyShapeshift:|
|8676|Ambush|FAIL|d:fixture:OnlyShapeshift:|
|8679|Wound Poison|PASS||
|14983|Vigor|PASS||
|27576|Mutilate|PASS||
|31224|Cloak of Shadows|PASS||
|31230|Cheat Death|PASS||
|51667|Cut to the Chase|PASS||
|56814|Detection|PASS||
|57934|Tricks of the Trade|FAIL|d:fixture:self_with_exclude_caster|
|58423|Relentless Strikes|PASS||
|76808|Mastery: Executioner|PASS||
|79008|Elusiveness|PASS||
|82245|Parry|PASS||
|86092|Leather Specialization|FAIL|d:fixture:no_passive_aura|
|86392|Main Gauche|PASS||
|91023|Find Weakness|PASS||
|108208|Subterfuge|PASS||
|108209|Shadow Focus|PASS||
|113780|Deadly Poison|PASS||
|114014|Shuriken Toss|PASS||
|114018|Shroud of Concealment|FAIL|d:fixture:OnlyShapeshift:|
|121471|Shadow Blades|PASS||
|137034|Rogue|PASS||
|137035|Subtlety Rogue|PASS||
|157442|Critical Strikes|PASS||
|162697|Stat Negation Aura - Agility DPS|PASS||
|185311|Crimson Vial|PASS||
|185313|Shadow Dance|PASS||
|185314|Deepening Shadows|PASS||
|185438|Shadowstrike|FAIL|d:fixture:OnlyShapeshift:|
|193531|Deeper Stratagem|PASS||
|193537|Weaponmaster|PASS||
|193539|Alacrity|PASS||
|193546|Iron Stomach|PASS||
|196819|Eviscerate|PASS||
|196912|Shadow Techniques|PASS||
|196976|Master of Shadows|PASS||
|197835|Shuriken Storm|PASS||
|200733|Airborne Irritant|PASS||
|200758|Gloomblade|PASS||
|210108|Detection|PASS||
|226178|missing|FAIL|d:missing_store|
|231018|missing|FAIL|d:missing_store|
|231691|Improved Sprint|PASS||
|231719|Deadened Nerves|PASS||
|245687|Dark Shadow|PASS||
|257505|Shot in the Dark|PASS||
|277953|Night Terrors|PASS||
|280716|Leeching Poison|PASS||
|280719|Secret Technique|PASS||
|315496|Slice and Dice|PASS||
|315584|Instant Poison|PASS||
|319066|Improved Wound Poison|PASS||
|319175|Black Powder|PASS||
|319949|Improved Backstab|PASS||
|319951|Improved Shuriken Storm|PASS||
|343160|Premeditation|PASS||
|378427|Nimble Fingers|PASS||
|378436|Master Poisoner|PASS||
|378807|Shadowrunner|PASS||
|378813|Fleet Footed|PASS||
|378996|Recuperator|PASS||
|379005|Blackjack|PASS||
|381542|Deadly Precision|PASS||
|381543|Virulent Poisons|PASS||
|381620|Improved Ambush|PASS||
|381621|Tight Spender|PASS||
|381637|Atrophic Poison|PASS||
|381988|Swift Slasher|PASS||
|382015|The Rotten|PASS||
|382017|Veiltouched|PASS||
|382238|Lethality|PASS||
|382245|Cold Blooded Killer|PASS||
|382503|Quick Decisions|PASS||
|382504|Dark Brew|FAIL|a:aura:220|
|382505|The First Dance|PASS||
|382506|Replicating Shadows|PASS||
|382507|Shrouded in Darkness|PASS||
|382508|Planned Execution|PASS||
|382511|Shadowed Finishers|PASS||
|382512|Improved Find Weakness|PASS||
|382513|Without a Trace|PASS||
|382514|Fade to Nothing|PASS||
|382515|Cloaked in Shadows|PASS||
|382517|Deeper Daggers|PASS||
|382518|Perforated Veins|PASS||
|382524|Lingering Shadow|PASS||
|382525|Finality|PASS||
|382528|Danse Macabre|PASS||
|385722|Silent Storm|PASS||
|393970|Soothing Darkness|PASS||
|394320|Secret Stratagem|PASS||
|394930|Double Dance|PASS||
|394931|Thrill Seeking|PASS||
|394935|Shadowstep|PASS||
|400804|Unbreakable Stride|PASS||
|423647|Graceful Guile|PASS||
|423662|Stillshroud|PASS||
|423683|Featherfoot|PASS||
|423701|Superior Mixture|PASS||
|426555|Warning Signs|PASS||
|426563|Ephemeral Bond|PASS||
|426591|Goremaw's Bite|PASS||
|426594|Shadowcraft|PASS||
|428387|Terrifying Pace|PASS||
|428486|Exhilarating Execution|PASS||
|441146|Unseen Blade|PASS||
|441247|Smoke|PASS||
|441250|Mirrors|PASS||
|441263|Devious Distractions|PASS||
|441273|Surprising Strikes|PASS||
|441274|Disorienting Strikes|PASS||
|441321|Flawless Form|PASS||
|441346|Thousand Cuts|PASS||
|441359|Flickerstrike|PASS||
|441367|Nimble Flurry|PASS||
|441398|No Scruples|PASS||
|441403|So Tricky|PASS||
|441415|Don't Be Suspicious|PASS||
|441423|Coup de Grace|PASS||
|441429|Cloud Cover|PASS||
|452536|Hand of Fate|PASS||
|453428|Mean Streak|PASS||
|453457|Edge Case|PASS||
|454286|Tempted Fate|PASS||
|454419|Deal Fate|PASS||
|454432|Inexorable March|PASS||
|454433|Death's Arrival|PASS||
|454435|Destiny Defined|PASS||
|457022|Ethereal Cloak|PASS||
|457034|Bait and Switch|PASS||
|457052|Deathstalker's Mark|PASS||
|457054|Hunt Them Down|PASS||
|457055|Singular Focus|PASS||
|457056|Lingering Darkness|PASS||
|457057|Shadewalker|PASS||
|457058|Darkest Night|PASS||
|457062|Symbolic Victory|PASS||
|457063|Shroud of Night|PASS||
|457067|Momentum of Despair|PASS||
|457068|Follow the Blood|PASS||
|462103|Rogue|PASS||
|462105|Subtlety Rogue|PASS||
|469642|Death Perception|PASS||
|469779|Thistle Tea|PASS||
|470347|Supercharger|PASS||
|470668|Forced Induction|PASS||
|470669|Echoing Reprimand|PASS||
|1229376|Single-Button Assistant|PASS||
|1248774|Unshakeable Drive|PASS||
|1248785|Corrupt the Blood|PASS||
|1248793|Clear the Witnesses|PASS||
|1248970|Lucky Coin|PASS||
|1249190|Overflowing Purse|PASS||
|1249194|Delivered Doom|PASS||
|1249201|Chosen's Revelry|PASS||
|1249204|Rush to the Inevitable|PASS||
|1249215|Fate Intertwined|PASS||
|1256946|Subtlety Rogue|FAIL|a:aura:647|
|1264764|Shuriken Tornado|PASS||
|1265952|Potent Powder|PASS||
|1267182|Toxic Stiletto|PASS||
|1267210|Quick Fingers|PASS||
|1267216|Deep Cuts|PASS||
|1267220|Danger Sense|PASS||
|1272989|Precise Killer|PASS||
|1273017|Quietus Celeris|PASS||
|1273035|Mass Casualty|PASS||
|1276626|Hoodwink|PASS||
|1276630|Flashing Steel|PASS||
|1276679|Clever Combatant|PASS||
|1276809|Ravenholdt Mint|PASS||
|1276816|Controlled Chaos|PASS||
|1277030|Sometimes Lucky|PASS||
|1279401|Shuriken Storm|PASS||
|1279444|Improved Secret Technique|PASS||
|1281468|Umbral Edge|PASS||
|1293135|Sanguine Vial|PASS||
|1298826|Thistle Tea|PASS||

### Shaman / Elemental (262)

|Spell|Name|Result|Observed causes|
|---:|---|---|---|
|370|Purge|PASS||
|546|Water Walking|PASS||
|556|Astral Recall|FAIL|d:fixture:NotSupported:missing homebind|
|974|Earth Shield|PASS||
|1064|Chain Heal|PASS||
|2008|Ancestral Spirit|PASS||
|2484|Earthbind Totem|PASS||
|2645|Ghost Wolf|PASS||
|2825|Bloodlust|PASS||
|6196|Far Sight|FAIL|a:effect:72|
|8004|Healing Surge|PASS||
|8042|Earth Shock|PASS||
|8143|Tremor Totem|PASS||
|8349|missing|FAIL|d:missing_store|
|10444|Flametongue Attack|PASS||
|16166|Master of the Elements|PASS||
|20608|Reincarnation|PASS||
|30884|Nature's Guardian|PASS||
|32182|Heroism|PASS||
|51485|Earthgrab Totem|PASS||
|51490|Thunderstorm|PASS||
|51505|Lava Burst|PASS||
|51514|Hex|PASS||
|51886|Cleanse Spirit|PASS||
|57994|Wind Shear|PASS||
|58875|Spirit Walk|PASS||
|60188|Elemental Fury|PASS||
|61882|Earthquake|PASS||
|73899|Primal Strike|PASS||
|77478|Earthquake|PASS||
|77756|Lava Surge|PASS||
|79206|Spiritwalker's Grace|PASS||
|86108|Mail Specialization|FAIL|d:fixture:no_passive_aura|
|86529|missing|FAIL|d:missing_store|
|108271|Astral Shift|PASS||
|108287|Totemic Projection|PASS||
|114050|Ascendance|PASS||
|114911|Ancestral Guidance|PASS||
|115360|Windstrike Off-Hand|PASS||
|117013|Primal Elementalist|PASS||
|117014|Elemental Blast|PASS||
|123831|Block|PASS||
|137038|Shaman|PASS||
|137040|Elemental Shaman|PASS||
|162699|Stat Negation Aura - Intellect DPS|PASS||
|168534|Mastery: Elemental Overload|PASS||
|170374|Earthen Rage|PASS||
|187828|Maelstrom|PASS||
|188196|Lightning Bolt|PASS||
|188443|Chain Lightning|PASS||
|191634|Stormkeeper|PASS||
|191717|Fury of the Storms|PASS||
|191861|Power of the Maelstrom|PASS||
|192058|Capacitor Totem|PASS||
|192063|Gust of Wind|PASS||
|192077|Wind Rush Totem|PASS||
|192088|Graceful Spirit|PASS||
|192106|Lightning Shield|PASS||
|196840|Frost Shock|PASS||
|198103|Earth Elemental|PASS||
|204268|Voodoo Mastery|PASS||
|210689|Lightning Rod|PASS||
|210873|Hex|PASS||
|211004|Hex|PASS||
|211010|Hex|PASS||
|211015|Hex|PASS||
|216965|missing|FAIL|d:missing_store|
|226178|missing|FAIL|d:missing_store|
|231018|missing|FAIL|d:missing_store|
|260878|Spirit Wolf|PASS||
|265046|Static Charge|PASS||
|269352|Hex|PASS||
|273221|Aftershock|PASS||
|277778|Hex|PASS||
|277784|Hex|PASS||
|280609|missing|FAIL|d:missing_store|
|309328|Hex|PASS||
|318038|Flametongue Weapon|PASS||
|318044|Lightning Bolt|PASS||
|320125|missing|FAIL|d:missing_store|
|333919|Echo of the Elements|PASS||
|343725|Maelstrom|PASS||
|355630|Windveil|PASS||
|377933|Astral Bulwark|PASS||
|378075|Thunderous Paws|PASS||
|378077|Spiritwalker's Aegis|PASS||
|378081|Nature's Swiftness|PASS||
|378193|Primordial Fury|PASS||
|378211|Refreshing Waters|PASS||
|378255|Call of Fire|PASS||
|378266|Flames of the Cauldron|PASS||
|378773|Greater Purge|PASS||
|378776|Inundate|PASS||
|381647|Planes Traveler|PASS||
|381650|Elemental Warding|PASS||
|381655|Nature's Fury|PASS||
|381689|Brimming with Life|PASS||
|381707|Swelling Maelstrom|PASS||
|381708|Eye of the Storm|PASS||
|381726|Mountains Will Fall|PASS||
|381782|Searing Flames|PASS||
|381784|Flames of the Firelord|PASS||
|381867|Totemic Surge|PASS||
|381930|Mana Spring|PASS||
|381936|Flash of Lightning|PASS||
|382032|Echo Chamber|PASS||
|382197|Ancestral Wolf Affinity|PASS||
|382201|Totemic Focus|PASS||
|382215|Winds of Al'Akir|PASS||
|382886|Fire and Ice|PASS||
|383010|Elemental Orbit|PASS||
|383013|Poison Cleansing Totem|PASS||
|392915|Healing Stream Totem|PASS||
|443418|Elemental Reverb|PASS||
|443423|Ancient Fellowship|PASS||
|443425|Spiritwalker's Momentum|PASS||
|443441|Earthen Communion|PASS||
|443442|Natural Harmony|PASS||
|443444|Heed My Call|PASS||
|443445|Routine Communication|PASS||
|443446|Final Calling|PASS||
|443447|Maelstrom Supremacy|PASS||
|443448|Primordial Capacity|PASS||
|443449|Latent Wisdom|PASS||
|443450|Call of the Ancestors|PASS||
|443451|Offering from Beyond|PASS||
|445024|Whirling Elements|PASS||
|445025|Totemic Rebound|PASS||
|445026|Oversized Totems|PASS||
|445027|Swift Recall|PASS||
|445028|Imbuement Mastery|PASS||
|445029|Amplification Core|PASS||
|445030|Oversurge|PASS||
|445031|Wind Barrier|PASS||
|445032|Pulse Capacitor|PASS||
|445033|Supportive Imbuements|PASS||
|445034|Lively Totems|PASS||
|445035|Splitstream|PASS||
|445036|Totemic Coordination|PASS||
|448861|Ancestral Swiftness|PASS||
|454009|Tempest|PASS||
|454021|Stormcaller|PASS||
|454022|Electroshock|PASS||
|454026|Rolling Thunder|PASS||
|454027|Nature's Protection|PASS||
|454372|Surging Currents|PASS||
|454391|Unlimited Power|PASS||
|454919|Voltaic Surge|PASS||
|455088|Storm Swell|PASS||
|455096|Arc Discharge|PASS||
|455110|Supercharge|PASS||
|455123|Conductive Energy|PASS||
|455129|Awakening Storms|PASS||
|455590|Earthsurge|PASS||
|455630|Surging Totem|PASS||
|462106|Shaman|FAIL|a:aura:320|
|462107|Elemental Shaman|PASS||
|462368|Elemental Resistance|PASS||
|462440|First Ascendant|PASS||
|462443|Preeminence|PASS||
|462454|Reactive Warding|PASS||
|462620|Earthquake|PASS||
|462695|Storm Frenzy|PASS||
|462757|Thunderstrike Ward|PASS||
|462762|Encasing Cold|PASS||
|462764|Arctic Snowstorm|PASS||
|462791|Ascending Air|PASS||
|462796|Enhanced Imbues|PASS||
|462817|Jet Stream|PASS||
|462840|Fusion of Elements|PASS||
|462854|Skyfury|PASS||
|462862|Lightning Capacitor|PASS||
|462866|Elemental Unity|PASS||
|462867|Everlasting Elements|PASS||
|467778|Lightning Conduit|PASS||
|468571|Herald of the Storms|PASS||
|468625|Charged Conduit|PASS||
|468626|Earthshatter|PASS||
|470057|Voltaic Blaze|PASS||
|470411|Flame Shock|PASS||
|1217622|Therazane's Resilience|PASS||
|1229376|Single-Button Assistant|PASS||
|1231772|Elemental Shaman|FAIL|a:aura:647; a:aura:648|
|1258843|Molten Wrath|PASS||
|1258889|Storm Infusion|PASS||
|1258895|Elemental Resonance|PASS||
|1258899|Tectonic Collapse|PASS||
|1259047|Inferno Arc|PASS||
|1259471|Purging Flames|PASS||
|1260644|Totemic Momentum|PASS||
|1260874|Primal Catalyst|PASS||
|1263288|Elemental Attunement|PASS||
|1264688|Descending Skies|PASS||
|1264691|Natural Gift|PASS||
|1264762|Stormwell|PASS||
|1269215|Crackling Fury|PASS||
|1269360|Amped Up|PASS||
|1269364|Path of the Seer|PASS||
|1270350|Instinctive Imbuements|PASS||
|1270375|Spiritual Awakening|PASS||
|1270446|Ancestral Influence|PASS||
|1270447|Windspeaker|PASS||
|1270450|Mystic Knowledge|PASS||
|1273485|Lava Flows|PASS||
|1279819|Primordial Bond|PASS||

### Shaman / Enhancement (263)

|Spell|Name|Result|Observed causes|
|---:|---|---|---|
|370|Purge|PASS||
|546|Water Walking|PASS||
|556|Astral Recall|FAIL|d:fixture:NotSupported:missing homebind|
|974|Earth Shield|PASS||
|1064|Chain Heal|PASS||
|2008|Ancestral Spirit|PASS||
|2484|Earthbind Totem|PASS||
|2645|Ghost Wolf|PASS||
|2825|Bloodlust|PASS||
|6196|Far Sight|FAIL|a:effect:72|
|8004|Healing Surge|PASS||
|8143|Tremor Totem|PASS||
|8349|missing|FAIL|d:missing_store|
|10444|Flametongue Attack|PASS||
|17364|Stormstrike|PASS||
|20608|Reincarnation|PASS||
|30884|Nature's Guardian|PASS||
|32182|Heroism|PASS||
|33757|Windfury Weapon|PASS||
|51485|Earthgrab Totem|PASS||
|51514|Hex|PASS||
|51886|Cleanse Spirit|PASS||
|57994|Wind Shear|PASS||
|58875|Spirit Walk|PASS||
|60103|Lava Lash|PASS||
|73899|Primal Strike|PASS||
|77223|Mastery: Enhanced Elements|PASS||
|77478|Earthquake|PASS||
|79206|Spiritwalker's Grace|PASS||
|86099|Mail Specialization|FAIL|d:fixture:no_passive_aura|
|86529|missing|FAIL|d:missing_store|
|86629|Dual Wield|PASS||
|108271|Astral Shift|PASS||
|108287|Totemic Projection|PASS||
|114051|Ascendance|FAIL|a:aura:367|
|114911|Ancestral Guidance|PASS||
|115360|Windstrike Off-Hand|PASS||
|123831|Block|PASS||
|137038|Shaman|PASS||
|137041|Enhancement Shaman|PASS||
|157444|Critical Strikes|PASS||
|162697|Stat Negation Aura - Agility DPS|PASS||
|187874|Crash Lightning|PASS||
|187880|Maelstrom Weapon|PASS||
|188196|Lightning Bolt|PASS||
|188443|Chain Lightning|PASS||
|192058|Capacitor Totem|PASS||
|192063|Gust of Wind|PASS||
|192077|Wind Rush Totem|PASS||
|192088|Graceful Spirit|PASS||
|192106|Lightning Shield|PASS||
|196840|Frost Shock|PASS||
|196884|Feral Lunge|PASS||
|197214|Sundering|FAIL|b:target:107|
|198103|Earth Elemental|PASS||
|201845|Stormsurge|PASS||
|201900|Hot Hand|PASS||
|204268|Voodoo Mastery|PASS||
|210853|Elemental Assault|PASS||
|210873|Hex|PASS||
|211004|Hex|PASS||
|211010|Hex|PASS||
|211015|Hex|PASS||
|216965|missing|FAIL|d:missing_store|
|226178|missing|FAIL|d:missing_store|
|231018|missing|FAIL|d:missing_store|
|260878|Spirit Wolf|PASS||
|262647|Forceful Winds|PASS||
|265046|Static Charge|PASS||
|269352|Hex|PASS||
|277778|Hex|PASS||
|277784|Hex|PASS||
|289874|Ride the Lightning|PASS||
|309328|Hex|PASS||
|318038|Flametongue Weapon|PASS||
|318044|Lightning Bolt|PASS||
|319930|Stormblast|PASS||
|320125|missing|FAIL|d:missing_store|
|334033|Molten Assault|PASS||
|334046|Lashing Flames|PASS||
|334308|Chaining Storms|PASS||
|344357|Stormflurry|PASS||
|355630|Windveil|PASS||
|377933|Astral Bulwark|PASS||
|378075|Thunderous Paws|PASS||
|378077|Spiritwalker's Aegis|PASS||
|378081|Nature's Swiftness|PASS||
|378211|Refreshing Waters|PASS||
|378270|Deeply Rooted Elements|PASS||
|378773|Greater Purge|PASS||
|381647|Planes Traveler|PASS||
|381650|Elemental Warding|PASS||
|381655|Nature's Fury|PASS||
|381689|Brimming with Life|PASS||
|381867|Totemic Surge|PASS||
|381930|Mana Spring|PASS||
|382042|Surging Elements|PASS||
|382197|Ancestral Wolf Affinity|PASS||
|382201|Totemic Focus|PASS||
|382215|Winds of Al'Akir|PASS||
|382886|Fire and Ice|PASS||
|382888|Flurry|PASS||
|383010|Elemental Orbit|PASS||
|383013|Poison Cleansing Totem|PASS||
|384143|Overflowing Maelstrom|PASS||
|384149|Raging Maelstrom|PASS||
|384352|Doom Winds|PASS||
|384355|Elemental Weapons|PASS||
|384363|Converging Storms|PASS||
|384411|Static Accumulation|PASS||
|384444|Thorim's Invocation|PASS||
|384450|Lightning Strikes|PASS||
|390288|Unruly Winds|PASS||
|390370|Ashen Catalyst|PASS||
|392352|Storm's Wrath|PASS||
|392915|Healing Stream Totem|PASS||
|443418|Elemental Reverb|PASS||
|443423|Ancient Fellowship|PASS||
|443425|Spiritwalker's Momentum|PASS||
|443441|Earthen Communion|PASS||
|443442|Natural Harmony|PASS||
|443444|Heed My Call|PASS||
|443445|Routine Communication|PASS||
|443446|Final Calling|PASS||
|443447|Maelstrom Supremacy|PASS||
|443448|Primordial Capacity|PASS||
|443449|Latent Wisdom|PASS||
|443450|Call of the Ancestors|PASS||
|443451|Offering from Beyond|PASS||
|445024|Whirling Elements|PASS||
|445025|Totemic Rebound|PASS||
|445026|Oversized Totems|PASS||
|445027|Swift Recall|PASS||
|445028|Imbuement Mastery|PASS||
|445029|Amplification Core|PASS||
|445030|Oversurge|PASS||
|445031|Wind Barrier|PASS||
|445032|Pulse Capacitor|PASS||
|445033|Supportive Imbuements|PASS||
|445034|Lively Totems|PASS||
|445035|Splitstream|PASS||
|445036|Totemic Coordination|PASS||
|448861|Ancestral Swiftness|PASS||
|454009|Tempest|PASS||
|454021|Stormcaller|PASS||
|454022|Electroshock|PASS||
|454026|Rolling Thunder|PASS||
|454027|Nature's Protection|PASS||
|454372|Surging Currents|PASS||
|454391|Unlimited Power|PASS||
|454919|Voltaic Surge|PASS||
|455088|Storm Swell|PASS||
|455096|Arc Discharge|PASS||
|455110|Supercharge|PASS||
|455123|Conductive Energy|PASS||
|455129|Awakening Storms|PASS||
|455590|Earthsurge|PASS||
|455630|Surging Totem|PASS||
|462106|Shaman|FAIL|a:aura:320|
|462108|Enhancement Shaman|PASS||
|462368|Elemental Resistance|PASS||
|462454|Reactive Warding|PASS||
|462762|Encasing Cold|PASS||
|462764|Arctic Snowstorm|PASS||
|462791|Ascending Air|PASS||
|462796|Enhanced Imbues|PASS||
|462817|Jet Stream|PASS||
|462854|Skyfury|PASS||
|467778|Lightning Conduit|PASS||
|469314|Feral Spirit|PASS||
|470057|Voltaic Blaze|PASS||
|470411|Flame Shock|PASS||
|1214207|Enhancement Shaman|FAIL|a:aura:647; a:aura:649|
|1217622|Therazane's Resilience|PASS||
|1218047|Primordial Storm|PASS||
|1229376|Single-Button Assistant|PASS||
|1250364|Elemental Tempo|PASS||
|1251026|Overcharge|PASS||
|1251069|Stormbind|PASS||
|1260644|Totemic Momentum|PASS||
|1260666|Fire Nova|PASS||
|1260874|Primal Catalyst|PASS||
|1262635|Thunder Capacitor|PASS||
|1263288|Elemental Attunement|PASS||
|1264688|Descending Skies|PASS||
|1264691|Natural Gift|PASS||
|1264762|Stormwell|PASS||
|1270350|Instinctive Imbuements|PASS||
|1270375|Spiritual Awakening|PASS||
|1270446|Ancestral Influence|PASS||
|1270447|Windspeaker|PASS||
|1270450|Mystic Knowledge|PASS||
|1279819|Primordial Bond|PASS||

### Shaman / Restoration (264)

|Spell|Name|Result|Observed causes|
|---:|---|---|---|
|370|Purge|PASS||
|546|Water Walking|PASS||
|556|Astral Recall|FAIL|d:fixture:NotSupported:missing homebind|
|974|Earth Shield|PASS||
|1064|Chain Heal|PASS||
|2008|Ancestral Spirit|PASS||
|2484|Earthbind Totem|PASS||
|2645|Ghost Wolf|PASS||
|2825|Bloodlust|PASS||
|6196|Far Sight|FAIL|a:effect:72|
|8004|Healing Surge|PASS||
|8143|Tremor Totem|PASS||
|8349|missing|FAIL|d:missing_store|
|10444|Flametongue Attack|PASS||
|16196|Resurgence|PASS||
|20608|Reincarnation|PASS||
|30884|Nature's Guardian|PASS||
|32182|Heroism|PASS||
|51485|Earthgrab Totem|PASS||
|51505|Lava Burst|PASS||
|51514|Hex|PASS||
|51564|Tidal Waves|PASS||
|52127|Water Shield|FAIL|d:fixture:logger_external_consumer:85|
|57994|Wind Shear|PASS||
|58875|Spirit Walk|PASS||
|61295|Riptide|PASS||
|73685|Unleash Life|PASS||
|73899|Primal Strike|PASS||
|73920|Healing Rain|PASS||
|77130|Purify Spirit|PASS||
|77226|Mastery: Deep Healing|PASS||
|77472|Healing Wave|PASS||
|77478|Earthquake|PASS||
|77756|Lava Surge|PASS||
|79206|Spiritwalker's Grace|PASS||
|86100|Mail Specialization|FAIL|d:fixture:no_passive_aura|
|86529|missing|FAIL|d:missing_store|
|98008|Spirit Link Totem|PASS||
|108271|Astral Shift|PASS||
|108280|Healing Tide Totem|PASS||
|108287|Totemic Projection|PASS||
|114052|Ascendance|PASS||
|114911|Ancestral Guidance|PASS||
|115360|Windstrike Off-Hand|PASS||
|123831|Block|PASS||
|137038|Shaman|PASS||
|137039|Restoration Shaman|PASS||
|162701|Stat Negation Aura - Intellect Healer|PASS||
|188196|Lightning Bolt|PASS||
|188443|Chain Lightning|PASS||
|192058|Capacitor Totem|PASS||
|192063|Gust of Wind|PASS||
|192077|Wind Rush Totem|PASS||
|192088|Graceful Spirit|PASS||
|192106|Lightning Shield|PASS||
|196840|Frost Shock|PASS||
|198103|Earth Elemental|PASS||
|200072|Torrent|PASS||
|200076|Deluge|PASS||
|204268|Voodoo Mastery|PASS||
|207401|Ancestral Vigor|PASS||
|210873|Hex|PASS||
|211004|Hex|PASS||
|211010|Hex|PASS||
|211015|Hex|PASS||
|212048|Ancestral Vision|PASS||
|216965|missing|FAIL|d:missing_store|
|226178|missing|FAIL|d:missing_store|
|231018|missing|FAIL|d:missing_store|
|260878|Spirit Wolf|PASS||
|265046|Static Charge|PASS||
|269352|Hex|PASS||
|277778|Hex|PASS||
|277784|Hex|PASS||
|309328|Hex|PASS||
|318044|Lightning Bolt|PASS||
|320125|missing|FAIL|d:missing_store|
|333919|Echo of the Elements|PASS||
|355630|Windveil|PASS||
|377933|Astral Bulwark|PASS||
|378075|Thunderous Paws|PASS||
|378077|Spiritwalker's Aegis|PASS||
|378081|Nature's Swiftness|PASS||
|378211|Refreshing Waters|PASS||
|378270|Deeply Rooted Elements|PASS||
|378443|Acid Rain|PASS||
|378773|Greater Purge|PASS||
|381647|Planes Traveler|PASS||
|381650|Elemental Warding|PASS||
|381655|Nature's Fury|PASS||
|381689|Brimming with Life|PASS||
|381867|Totemic Surge|PASS||
|381930|Mana Spring|PASS||
|381946|Wavespeaker's Blessing|PASS||
|382020|Earthen Harmony|PASS||
|382021|Earthliving Weapon|PASS||
|382030|Water Totem Mastery|PASS||
|382039|Flow of the Tides|PASS||
|382045|Primal Tide Core|PASS||
|382194|Undercurrent|PASS||
|382197|Ancestral Wolf Affinity|PASS||
|382201|Totemic Focus|PASS||
|382215|Winds of Al'Akir|PASS||
|382309|Ancestral Awakening|PASS||
|382315|Improved Earthliving Weapon|PASS||
|382482|Living Stream|PASS||
|382732|Ancestral Reach|PASS||
|382886|Fire and Ice|PASS||
|383010|Elemental Orbit|PASS||
|383013|Poison Cleansing Totem|PASS||
|383016|Improved Purify Spirit|PASS||
|383222|Overflowing Shores|PASS||
|392915|Healing Stream Totem|PASS||
|392916|Healing Stream Totem|PASS||
|440012|Cleanse Spirit|PASS||
|443418|Elemental Reverb|PASS||
|443423|Ancient Fellowship|PASS||
|443425|Spiritwalker's Momentum|PASS||
|443441|Earthen Communion|PASS||
|443442|Natural Harmony|PASS||
|443444|Heed My Call|PASS||
|443445|Routine Communication|PASS||
|443446|Final Calling|PASS||
|443447|Maelstrom Supremacy|PASS||
|443448|Primordial Capacity|PASS||
|443449|Latent Wisdom|PASS||
|443450|Call of the Ancestors|PASS||
|443451|Offering from Beyond|PASS||
|445024|Whirling Elements|PASS||
|445025|Totemic Rebound|PASS||
|445026|Oversized Totems|PASS||
|445027|Swift Recall|PASS||
|445028|Imbuement Mastery|PASS||
|445029|Amplification Core|PASS||
|445030|Oversurge|PASS||
|445031|Wind Barrier|PASS||
|445032|Pulse Capacitor|PASS||
|445033|Supportive Imbuements|PASS||
|445034|Lively Totems|PASS||
|445035|Splitstream|PASS||
|445036|Totemic Coordination|PASS||
|448861|Ancestral Swiftness|PASS||
|454009|Tempest|PASS||
|454021|Stormcaller|PASS||
|454022|Electroshock|PASS||
|454026|Rolling Thunder|PASS||
|454027|Nature's Protection|PASS||
|454372|Surging Currents|PASS||
|454391|Unlimited Power|PASS||
|454919|Voltaic Surge|PASS||
|455088|Storm Swell|PASS||
|455096|Arc Discharge|PASS||
|455110|Supercharge|PASS||
|455123|Conductive Energy|PASS||
|455129|Awakening Storms|PASS||
|455590|Earthsurge|PASS||
|455630|Surging Totem|PASS||
|462106|Shaman|FAIL|a:aura:320|
|462110|Restoration Shaman|PASS||
|462368|Elemental Resistance|PASS||
|462383|Spouting Spirits|PASS||
|462424|Tidewaters|PASS||
|462440|First Ascendant|PASS||
|462443|Preeminence|PASS||
|462454|Reactive Warding|PASS||
|462486|Downpour|PASS||
|462587|White Water|FAIL|a:aura:50|
|462762|Encasing Cold|PASS||
|462764|Arctic Snowstorm|PASS||
|462791|Ascending Air|PASS||
|462796|Enhanced Imbues|PASS||
|462817|Jet Stream|PASS||
|462854|Skyfury|PASS||
|467778|Lightning Conduit|PASS||
|470076|Coalescing Water|PASS||
|470411|Flame Shock|PASS||
|472916|missing|FAIL|d:missing_store|
|1217622|Therazane's Resilience|PASS||
|1229376|Single-Button Assistant|PASS||
|1252874|Soothing Rain|PASS||
|1252882|Double Dip|PASS||
|1253014|Water Expulsion|PASS||
|1253090|Crashing Waves|PASS||
|1253093|Current Control|PASS||
|1253099|Quickstream|PASS||
|1254210|Earthweaver|PASS||
|1254251|Rip Current|PASS||
|1256945|Restoration Shaman|FAIL|a:aura:647; a:aura:649|
|1260644|Totemic Momentum|PASS||
|1260874|Primal Catalyst|PASS||
|1263288|Elemental Attunement|PASS||
|1264688|Descending Skies|PASS||
|1264691|Natural Gift|PASS||
|1264762|Stormwell|PASS||
|1270350|Instinctive Imbuements|PASS||
|1270375|Spiritual Awakening|PASS||
|1270446|Ancestral Influence|PASS||
|1270447|Windspeaker|PASS||
|1270450|Mystic Knowledge|PASS||
|1271104|Earthen Accord|PASS||
|1279819|Primordial Bond|PASS||
|1312843|Swelling Tides|PASS||

### Warlock / Affliction (265)

|Spell|Name|Result|Observed causes|
|---:|---|---|---|
|126|Eye of Kilrogg|PASS||
|172|Corruption|PASS||
|686|Shadow Bolt|PASS||
|688|Summon Imp|PASS||
|691|Summon Felhunter|PASS||
|697|Summon Voidwalker|PASS||
|698|Ritual of Summoning|PASS||
|702|Curse of Weakness|PASS||
|710|Banish|FAIL|d:fixture:ineligible_creature_type|
|980|Agony|PASS||
|1098|Subjugate Demon|FAIL|d:fixture:ineligible_creature_type|
|1714|Curse of Tongues|FAIL|a:aura:355; a:aura:75|
|5484|Howl of Terror|PASS||
|5697|Unending Breath|PASS||
|5782|Fear|PASS||
|5784|Felsteed|PASS||
|6201|Create Healthstone|PASS||
|6789|Mortal Coil|PASS||
|20707|Soulstone|FAIL|b:target:121|
|23161|Dreadsteed|PASS||
|27243|Seed of Corruption|PASS||
|29893|Create Soulwell|PASS||
|30283|Shadowfury|PASS||
|48181|Haunt|FAIL|a:aura:381|
|56244|Horrify|PASS||
|77215|Mastery: Potent Afflictions|PASS||
|86091|Nethermancy|FAIL|d:fixture:no_passive_aura|
|93375|Control Demon|PASS||
|101508|The Codex of Xerrath|FAIL|a:aura:403|
|104773|Unending Resolve|PASS||
|108366|Soul Leech|PASS||
|108415|Soul Link|PASS||
|108416|Dark Pact|PASS||
|108503|Grimoire of Sacrifice|PASS||
|108558|Nightfall|PASS||
|111400|Burning Rush|PASS||
|111771|Demonic Gateway|PASS||
|114190|missing|FAIL|d:missing_store|
|119898|Command Demon|FAIL|d:fixture:InvalidTarget:SPELL_FAILED_CASTER_AURASTATE|
|137042|Warlock|PASS||
|137043|Affliction Warlock|PASS||
|146739|Corruption|PASS||
|162699|Stat Negation Aura - Intellect DPS|PASS||
|196103|Absolute Corruption|PASS||
|196226|Sow the Seeds|PASS||
|205180|Summon Darkglare|PASS||
|219272|Demon Skin|PASS||
|226178|missing|FAIL|d:missing_store|
|231018|missing|FAIL|d:missing_store|
|231811|Soulstone|PASS||
|234153|Drain Life|PASS||
|234876|Death's Embrace|PASS||
|246985|Soul Shards|PASS||
|257190|missing|FAIL|d:missing_store|
|264000|Creeping Death|PASS||
|268358|Demonic Circle|FAIL|d:fixture:logger_external_consumer:195|
|288843|Demonic Embrace|PASS||
|317138|Strength of Will|PASS||
|333889|Fel Domination|PASS||
|334275|Curse of Exhaustion|PASS||
|342601|Ritual of Doom|PASS||
|342914|Fear|PASS||
|366222|Summon Sayaad|PASS||
|385881|Teachings of the Black Harvest|PASS||
|385899|Soulburn|PASS||
|386110|Fiendish Stride|PASS||
|386113|Fel Pact|PASS||
|386124|Fel Armor|PASS||
|386617|Demonic Fortitude|FAIL|a:aura:382|
|386648|Nightmare|PASS||
|386659|Dark Accord|PASS||
|386664|Ichor of Devils|PASS||
|386686|Frequent Donor|PASS||
|386689|Pact of Gluttony|PASS||
|386976|Withering Bolt|PASS||
|386986|Sacrolash's Dark Strike|PASS||
|388667|Drain Soul|PASS||
|389367|Fel Synergy|PASS||
|389590|Demonic Resilience|FAIL|d:fixture:logger_external_consumer:197|
|389609|Abyss Walker|PASS||
|389623|Gorefiend's Resolve|PASS||
|416615|Xavius' Gambit|PASS||
|430014|Malevolence|PASS||
|440040|Xalan's Cruelty|PASS||
|440043|Blackened Soul|PASS||
|440044|Xalan's Ferocity|PASS||
|440045|Mark of Peroth'arn|PASS||
|440046|Mark of Xavius|PASS||
|440048|Hatefury Rituals|PASS||
|440051|Bleakheart Tactics|PASS||
|440055|Seeds of Their Demise|PASS||
|440057|Curse of the Satyr|PASS||
|440059|Aura of Enfeeblement|PASS||
|440065|Zevrim's Resilience|PASS||
|440070|Illhoof's Design|PASS||
|445465|Wither|PASS||
|449614|Demonic Soul|PASS||
|449620|Necrolyte Teachings|PASS||
|449624|Soul Anathema|PASS||
|449629|Demoniac's Fervor|PASS||
|449631|Wicked Reaping|PASS||
|449634|Quietus|PASS||
|449637|Sataiel's Volition|PASS||
|449638|Shadow of Death|PASS||
|449701|Gorebound Fortitude|PASS||
|449703|Friends In Dark Places|PASS||
|449704|Shared Fate|PASS||
|449706|Feast of Souls|PASS||
|449707|Eternal Servitude|PASS||
|452902|Swift Artifice|PASS||
|452999|Siphon Life|PASS||
|453080|Improved Shadow Bolt|PASS||
|453087|Malediction|PASS||
|453096|Contagion|PASS||
|453105|Summoner's Embrace|PASS||
|453172|Cunning Cruelty|PASS||
|458034|Improved Haunt|PASS||
|459440|Ravenous Afflictions|PASS||
|462111|Affliction Warlock|PASS||
|462112|Warlock|FAIL|a:aura:320|
|1229376|Single-Button Assistant|PASS||
|1256906|Affliction Warlock|FAIL|a:aura:647; a:aura:648; a:aura:649|
|1257052|Dark Harvest|PASS||
|1259790|Unstable Affliction|PASS||
|1259811|Practiced Pestilence|PASS||
|1259815|Potent Soul Shards|PASS||
|1259825|Shared Agony|PASS||
|1259838|Seeds of Destruction|PASS||
|1259886|Cull the Weak|PASS||
|1260209|Sudden Onset|PASS||
|1260229|Fatal Echoes|PASS||
|1260264|Shard Instability|PASS||
|1261124|Cascading Calamity|PASS||
|1261149|Malefic Grasp|PASS||
|1265799|Blight of Weakness|PASS||
|1265801|Frequent Traveler|PASS||
|1265810|Infernal Beneficiary|PASS||
|1265813|Foul Mouth|PASS||
|1265816|Improved Mortal Coil|PASS||
|1266799|Through the Felvine|PASS||
|1266803|Alzzin's Iniquity|PASS||
|1266805|Devil Fruit|PASS||
|1268884|Manifested Avarice|PASS||
|1268889|Shared Vessel|PASS||
|1268903|Eternal Hunger|PASS||
|1270255|Oppressive Darkness|PASS||
|1270690|Pact of the Nathrezim|FAIL|a:aura:443|
|1270691|Pact of the Satyr|PASS||
|1270693|Pact of the Annihilan|PASS||
|1270695|Pact of the Eredar|PASS||
|1270701|Gorefiend's Avarice|PASS||
|1271689|Empowered Drain Life|PASS||
|1271694|Fortified Soul|PASS||
|1271699|Empowered Healthstone|PASS||
|1271802|Blight of Tongues|PASS||
|1279510|Niskaran Methods|PASS||
|1279521|Eye Contract|PASS||
|1280733|Nether Plating|PASS||
|1311653|Soul Leech|PASS||
|1311969|Hedonic Gorging|PASS||
|1312998|Impetuous Wrath|PASS||

### Warlock / Demonology (266)

|Spell|Name|Result|Observed causes|
|---:|---|---|---|
|126|Eye of Kilrogg|PASS||
|172|Corruption|PASS||
|686|Shadow Bolt|PASS||
|688|Summon Imp|PASS||
|691|Summon Felhunter|PASS||
|697|Summon Voidwalker|PASS||
|698|Ritual of Summoning|PASS||
|702|Curse of Weakness|PASS||
|710|Banish|FAIL|d:fixture:ineligible_creature_type|
|1098|Subjugate Demon|FAIL|d:fixture:ineligible_creature_type|
|1714|Curse of Tongues|FAIL|a:aura:355; a:aura:75|
|5484|Howl of Terror|PASS||
|5697|Unending Breath|PASS||
|5782|Fear|PASS||
|5784|Felsteed|PASS||
|6201|Create Healthstone|PASS||
|6789|Mortal Coil|PASS||
|20707|Soulstone|FAIL|b:target:121|
|23161|Dreadsteed|PASS||
|29893|Create Soulwell|PASS||
|30146|Summon Felguard|PASS||
|30283|Shadowfury|PASS||
|56244|Horrify|PASS||
|77219|Mastery: Master Demonologist|PASS||
|86091|Nethermancy|FAIL|d:fixture:no_passive_aura|
|93375|Control Demon|PASS||
|101508|The Codex of Xerrath|FAIL|a:aura:403|
|104316|Call Dreadstalkers|PASS||
|104773|Unending Resolve|PASS||
|108366|Soul Leech|PASS||
|108415|Soul Link|PASS||
|108416|Dark Pact|PASS||
|111400|Burning Rush|PASS||
|111771|Demonic Gateway|PASS||
|114190|missing|FAIL|d:missing_store|
|119898|Command Demon|FAIL|d:fixture:InvalidTarget:SPELL_FAILED_CASTER_AURASTATE|
|137042|Warlock|PASS||
|137044|Demonology Warlock|PASS||
|146739|Corruption|PASS||
|162699|Stat Negation Aura - Intellect DPS|PASS||
|196277|Implosion|FAIL|d:fixture:InvalidTarget:SPELL_FAILED_CASTER_AURASTATE|
|219272|Demon Skin|PASS||
|226178|missing|FAIL|d:missing_store|
|231018|missing|FAIL|d:missing_store|
|231811|Soulstone|PASS||
|234153|Drain Life|PASS||
|246985|Soul Shards|PASS||
|257190|missing|FAIL|d:missing_store|
|264078|Dreadlash|PASS||
|264130|Power Siphon|FAIL|d:fixture:InvalidTarget:SPELL_FAILED_CASTER_AURASTATE|
|265187|Summon Demonic Tyrant|PASS||
|267214|Sacrificed Souls|PASS||
|267216|Inner Demons|PASS||
|268358|Demonic Circle|FAIL|d:fixture:logger_external_consumer:195|
|288843|Demonic Embrace|PASS||
|317138|Strength of Will|PASS||
|333889|Fel Domination|PASS||
|334275|Curse of Exhaustion|PASS||
|334591|Fel Firebolt|PASS||
|342601|Ritual of Doom|PASS||
|342914|Fear|PASS||
|366222|Summon Sayaad|PASS||
|385881|Teachings of the Black Harvest|PASS||
|385899|Soulburn|PASS||
|386110|Fiendish Stride|PASS||
|386113|Fel Pact|PASS||
|386124|Fel Armor|PASS||
|386185|Demonic Knowledge|PASS||
|386194|Carnivorous Stalkers|PASS||
|386617|Demonic Fortitude|FAIL|a:aura:382|
|386648|Nightmare|PASS||
|386659|Dark Accord|PASS||
|386664|Ichor of Devils|PASS||
|386686|Frequent Donor|PASS||
|386689|Pact of Gluttony|PASS||
|389367|Fel Synergy|PASS||
|389590|Demonic Resilience|FAIL|d:fixture:logger_external_consumer:197|
|389609|Abyss Walker|PASS||
|389623|Gorefiend's Resolve|PASS||
|416230|Imp-erator|PASS||
|426115|Demoniac|PASS||
|428394|Spiteful Reconstitution|PASS||
|428514|Diabolic Ritual|PASS||
|428517|Cloven Souls|PASS||
|428518|Secrets of the Coven|PASS||
|428522|Ruination|PASS||
|428911|Soul-Etched Circles|PASS||
|429072|Annihilan's Bellow|PASS||
|429115|Infernal Vitality|PASS||
|429130|Infernal Bulwark|PASS||
|429581|Abyssal Dominion|FAIL|a:aura:382|
|429657|Flames of Xoroth|PASS||
|429893|Touch of Rancora|PASS||
|429899|Gloom of Nathreza|PASS||
|429902|Cruelty of Kerxan|PASS||
|429917|Infernal Machine|PASS||
|449614|Demonic Soul|PASS||
|449620|Necrolyte Teachings|PASS||
|449624|Soul Anathema|PASS||
|449629|Demoniac's Fervor|PASS||
|449631|Wicked Reaping|PASS||
|449634|Quietus|PASS||
|449637|Sataiel's Volition|PASS||
|449638|Shadow of Death|PASS||
|449701|Gorebound Fortitude|PASS||
|449703|Friends In Dark Places|PASS||
|449704|Shared Fate|PASS||
|449706|Feast of Souls|PASS||
|449707|Eternal Servitude|PASS||
|452902|Swift Artifice|PASS||
|453699|Flametouched|PASS||
|453744|Rune of Shadows|PASS||
|453800|Improved Demonic Tactics|PASS||
|453908|Demonic Brutality|PASS||
|455449|Mark of Shatug|PASS||
|455450|Mark of F'harg|PASS||
|460551|Doom|PASS||
|462112|Warlock|FAIL|a:aura:320|
|462113|Demonology Warlock|PASS||
|1229376|Single-Button Assistant|PASS||
|1240189|Master Summoner|PASS||
|1245325|Track Pets|PASS||
|1250273|Hand of Gul'dan|PASS||
|1250372|Fel Intellect|PASS||
|1250375|Practiced Rituals|PASS||
|1250768|Imp Gang Boss|PASS||
|1250897|Hellbent Commander|PASS||
|1250921|Antoran Armaments|PASS||
|1251778|Summon Vilefiend|PASS||
|1256907|Demonology Warlock|FAIL|a:aura:647; a:aura:648; a:aura:649|
|1263935|Fel Armaments|PASS||
|1263941|Infernal Rapidity|PASS||
|1265799|Blight of Weakness|PASS||
|1265801|Frequent Traveler|PASS||
|1265810|Infernal Beneficiary|PASS||
|1265813|Foul Mouth|PASS||
|1265816|Improved Mortal Coil|PASS||
|1268709|Diabolic Oculi|PASS||
|1268713|Looks That Kill|PASS||
|1268716|Mind's Eyes|PASS||
|1268884|Manifested Avarice|PASS||
|1268889|Shared Vessel|PASS||
|1268903|Eternal Hunger|PASS||
|1270255|Oppressive Darkness|PASS||
|1270690|Pact of the Nathrezim|FAIL|a:aura:443|
|1270691|Pact of the Satyr|PASS||
|1270693|Pact of the Annihilan|PASS||
|1270695|Pact of the Eredar|PASS||
|1270701|Gorefiend's Avarice|PASS||
|1271689|Empowered Drain Life|PASS||
|1271694|Fortified Soul|PASS||
|1271699|Empowered Healthstone|PASS||
|1271802|Blight of Tongues|PASS||
|1276433|Dominant Hand|PASS||
|1276452|Grimoire: Imp Lord|PASS||
|1276467|Grimoire: Fel Ravager|PASS||
|1276661|Stabilized Portals|PASS||
|1276672|Summon Doomguard|PASS||
|1276746|Tyrant's Oblation|PASS||
|1276748|Reign of Tyranny|PASS||
|1276947|Demonic Calling|PASS||
|1276956|Blighted Maw|PASS||
|1279575|Empowered Felstorm|PASS||
|1281511|To Hell and Back|PASS||
|1311653|Soul Leech|PASS||

### Warlock / Destruction (267)

|Spell|Name|Result|Observed causes|
|---:|---|---|---|
|126|Eye of Kilrogg|PASS||
|172|Corruption|PASS||
|348|Immolate|PASS||
|686|Shadow Bolt|PASS||
|688|Summon Imp|PASS||
|691|Summon Felhunter|PASS||
|697|Summon Voidwalker|PASS||
|698|Ritual of Summoning|PASS||
|702|Curse of Weakness|PASS||
|710|Banish|FAIL|d:fixture:ineligible_creature_type|
|1098|Subjugate Demon|FAIL|d:fixture:ineligible_creature_type|
|1122|Summon Infernal|FAIL|a:effect:148|
|1714|Curse of Tongues|FAIL|a:aura:355; a:aura:75|
|5484|Howl of Terror|PASS||
|5697|Unending Breath|PASS||
|5740|Rain of Fire|PASS||
|5782|Fear|PASS||
|5784|Felsteed|PASS||
|6201|Create Healthstone|PASS||
|6353|Soul Fire|PASS||
|6789|Mortal Coil|PASS||
|17877|Shadowburn|PASS||
|17962|Conflagrate|PASS||
|20707|Soulstone|FAIL|b:target:121|
|23161|Dreadsteed|PASS||
|29722|Incinerate|PASS||
|29893|Create Soulwell|PASS||
|30283|Shadowfury|PASS||
|56244|Horrify|PASS||
|77220|Mastery: Chaotic Energies|PASS||
|80240|Havoc|PASS||
|86091|Nethermancy|FAIL|d:fixture:no_passive_aura|
|93375|Control Demon|PASS||
|101508|The Codex of Xerrath|FAIL|a:aura:403|
|104773|Unending Resolve|PASS||
|108366|Soul Leech|PASS||
|108415|Soul Link|PASS||
|108416|Dark Pact|PASS||
|108503|Grimoire of Sacrifice|PASS||
|111400|Burning Rush|PASS||
|111771|Demonic Gateway|PASS||
|114190|missing|FAIL|d:missing_store|
|116858|Chaos Bolt|PASS||
|119898|Command Demon|FAIL|d:fixture:InvalidTarget:SPELL_FAILED_CASTER_AURASTATE|
|137042|Warlock|PASS||
|137046|Destruction Warlock|PASS||
|146739|Corruption|PASS||
|152108|Cataclysm|PASS||
|157903|Warlock|PASS||
|162699|Stat Negation Aura - Intellect DPS|PASS||
|193541|Immolate|PASS||
|196406|Backdraft|PASS||
|196408|Fire and Brimstone|PASS||
|196447|Channel Demonfire|FAIL|d:fixture:InvalidTarget:SPELL_FAILED_CASTER_AURASTATE|
|205148|Reverse Entropy|PASS||
|219272|Demon Skin|PASS||
|226178|missing|FAIL|d:missing_store|
|231018|missing|FAIL|d:missing_store|
|231793|Improved Conflagrate|PASS||
|231811|Soulstone|PASS||
|234153|Drain Life|PASS||
|246985|Soul Shards|PASS||
|257190|missing|FAIL|d:missing_store|
|266086|Rain of Chaos|PASS||
|266134|Internal Combustion|PASS||
|268358|Demonic Circle|FAIL|d:fixture:logger_external_consumer:195|
|288843|Demonic Embrace|PASS||
|317138|Strength of Will|PASS||
|333889|Fel Domination|PASS||
|334275|Curse of Exhaustion|PASS||
|342601|Ritual of Doom|PASS||
|342914|Fear|PASS||
|366222|Summon Sayaad|PASS||
|385881|Teachings of the Black Harvest|PASS||
|385899|Soulburn|PASS||
|386110|Fiendish Stride|PASS||
|386113|Fel Pact|PASS||
|386124|Fel Armor|PASS||
|386617|Demonic Fortitude|FAIL|a:aura:382|
|386648|Nightmare|PASS||
|386659|Dark Accord|PASS||
|386664|Ichor of Devils|PASS||
|386686|Frequent Donor|PASS||
|386689|Pact of Gluttony|PASS||
|387103|Ruin|PASS||
|387108|Conflagration of Chaos|PASS||
|387166|Raging Demonfire|PASS||
|387173|Diabolic Embers|PASS||
|387252|Ashen Remains|PASS||
|387259|Flashpoint|PASS||
|387275|Chaos Incarnate|PASS||
|387384|Backlash|PASS||
|387506|Mayhem|PASS||
|388827|Explosive Potential|PASS||
|388832|Scalding Flames|PASS||
|389367|Fel Synergy|PASS||
|389590|Demonic Resilience|FAIL|d:fixture:logger_external_consumer:197|
|389609|Abyss Walker|PASS||
|417234|Crashing Chaos|PASS||
|428514|Diabolic Ritual|PASS||
|428517|Cloven Souls|PASS||
|428518|Secrets of the Coven|PASS||
|428522|Ruination|PASS||
|428911|Soul-Etched Circles|PASS||
|429072|Annihilan's Bellow|PASS||
|429115|Infernal Vitality|PASS||
|429130|Infernal Bulwark|PASS||
|429581|Abyssal Dominion|FAIL|a:aura:382|
|429657|Flames of Xoroth|PASS||
|429893|Touch of Rancora|PASS||
|429899|Gloom of Nathreza|PASS||
|429902|Cruelty of Kerxan|PASS||
|429917|Infernal Machine|PASS||
|430014|Malevolence|PASS||
|440040|Xalan's Cruelty|PASS||
|440043|Blackened Soul|PASS||
|440044|Xalan's Ferocity|PASS||
|440045|Mark of Peroth'arn|PASS||
|440046|Mark of Xavius|PASS||
|440048|Hatefury Rituals|PASS||
|440051|Bleakheart Tactics|PASS||
|440055|Seeds of Their Demise|PASS||
|440057|Curse of the Satyr|PASS||
|440059|Aura of Enfeeblement|PASS||
|440065|Zevrim's Resilience|PASS||
|440070|Illhoof's Design|PASS||
|445465|Wither|PASS||
|452902|Swift Artifice|PASS||
|453105|Summoner's Embrace|PASS||
|454735|Devastation|PASS||
|454744|Emberstorm|PASS||
|456951|Improved Chaos Bolt|PASS||
|462112|Warlock|FAIL|a:aura:320|
|462114|Destruction Warlock|PASS||
|1214442|Demonfire Infusion|PASS||
|1214467|Rain of Fire|FAIL|a:effect:353|
|1229376|Single-Button Assistant|PASS||
|1244284|Practiced Chaos|PASS||
|1244310|Roaring Blaze|PASS||
|1244460|Improved Havoc|PASS||
|1244788|Chaotic Inferno|PASS||
|1244877|Lake of Fire|PASS||
|1244928|Destructive Rapidity|PASS||
|1244941|Alythess's Ire|PASS||
|1245089|Avatar of Destruction|PASS||
|1245633|Fiendish Cruelty|PASS||
|1256908|Destruction Warlock|FAIL|a:aura:647; a:aura:649|
|1265799|Blight of Weakness|PASS||
|1265801|Frequent Traveler|PASS||
|1265810|Infernal Beneficiary|PASS||
|1265813|Foul Mouth|PASS||
|1265816|Improved Mortal Coil|PASS||
|1266799|Through the Felvine|PASS||
|1266803|Alzzin's Iniquity|PASS||
|1266805|Devil Fruit|PASS||
|1268709|Diabolic Oculi|PASS||
|1268713|Looks That Kill|PASS||
|1268716|Mind's Eyes|PASS||
|1270255|Oppressive Darkness|PASS||
|1270690|Pact of the Nathrezim|FAIL|a:aura:443|
|1270691|Pact of the Satyr|PASS||
|1270693|Pact of the Annihilan|PASS||
|1270695|Pact of the Eredar|PASS||
|1270701|Gorefiend's Avarice|PASS||
|1271689|Empowered Drain Life|PASS||
|1271694|Fortified Soul|PASS||
|1271699|Empowered Healthstone|PASS||
|1271802|Blight of Tongues|PASS||
|1280483|Inferno|PASS||
|1280868|Dimensional Rift|PASS||
|1311653|Soul Leech|PASS||

### Monk / Brewmaster (268)

|Spell|Name|Result|Observed causes|
|---:|---|---|---|
|8647|Mystic Touch|PASS||
|100780|Tiger Palm|PASS||
|100784|Blackout Kick|PASS||
|101546|Spinning Crane Kick|PASS||
|101643|Transcendence|PASS||
|107270|Spinning Crane Kick|PASS||
|107500|<DND> Monk Energy Driver|PASS||
|109132|Roll|PASS||
|115008|Chi Torpedo|FAIL|a:aura:373|
|115069|Stagger|PASS||
|115078|Paralysis|PASS||
|115173|Celerity|PASS||
|115175|Soothing Mist|PASS||
|115178|Resuscitate|PASS||
|115181|Breath of Fire|PASS||
|115315|Summon Black Ox Statue|PASS||
|115399|Black Ox Brew|PASS||
|115546|Provoke|FAIL|d:fixture:provoke_self|
|116095|Disable|PASS||
|116670|Vivify|PASS||
|116705|Spear Hand Strike|PASS||
|116812|Parry|PASS||
|116841|Tiger's Lust|PASS||
|116844|Ring of Peace|PASS||
|116847|Rushing Jade Wind|PASS||
|117418|Fists of Fury|PASS||
|117906|Mastery: Elusive Brawler|PASS||
|117952|Crackling Jade Lightning|PASS||
|117959|Crackling Jade Lightning|PASS||
|119381|Leg Sweep|PASS||
|119582|Purifying Brew|PASS||
|119611|Renewing Mist|PASS||
|119650|Energy Usage|PASS||
|120225|Leather Specialization|FAIL|d:fixture:no_passive_aura|
|121128|Touch of Death Notification Driver|PASS||
|121253|Keg Smash|PASS||
|123586|Flying Serpent Kick|PASS||
|123986|Chi Burst|PASS||
|124146|Dual Wield|PASS||
|124280|Touch of Karma|PASS||
|124502|Gift of the Ox|PASS||
|125355|Healing Sphere|PASS||
|125883|Zen Flight|FAIL|a:aura:206|
|126892|Zen Pilgrimage|PASS||
|126893|Zen Pilgrimage/Death Gate/Moonglade Storage Aura I|PASS||
|126895|Zen Pilgrimage: Return|PASS||
|130610|Monk|PASS||
|130654|Chi Burst|PASS||
|132463|Chi Wave|PASS||
|132467|Chi Wave|PASS||
|132578|Invoke Niuzao, the Black Ox|PASS||
|137022|Monk|PASS||
|137023|Brewmaster Monk|PASS||
|157361|Roll Speed Controls|PASS||
|157411|Windwalking|PASS||
|157445|Critical Strikes|PASS||
|162700|Stat Negation Aura - Agility Tank|PASS||
|169340|Touch of Fatality|PASS||
|196730|Special Delivery|PASS||
|196736|Blackout Combo|PASS||
|196737|High Tolerance|PASS||
|198898|Song of Chi-Ji|PASS||
|216519|Celestial Fortune|PASS||
|218164|Detox|PASS||
|226178|missing|FAIL|d:missing_store|
|231018|missing|FAIL|d:missing_store|
|231602|Improved Vivify|PASS||
|245013|Brewmaster's Balance|PASS||
|264348|Tiger Tail Sweep|PASS||
|280515|Bob and Weave|PASS||
|322101|Expel Harm|PASS||
|322102|Expel Harm|PASS||
|322109|Touch of Death|FAIL|d:fixture:healthy_touch_of_death_target|
|322113|Improved Touch of Death|PASS||
|322120|Shuffle|PASS||
|322507|Celestial Brew|PASS||
|322729|Spinning Crane Kick|PASS||
|322740|Mighty Stomp|PASS||
|322960|Fortifying Brew: Determination|PASS||
|325093|Light Brewing|PASS||
|325095|Touch of Death|PASS||
|325153|Exploding Keg|PASS||
|328670|Hasty Provocation|PASS||
|344359|Ancient Arts|PASS||
|346104|Windwalker Monk Two-Hand Adjustment|PASS||
|383695|Improved Blackout Kick|PASS||
|383697|Sal'salabim's Strength|PASS||
|383698|Scalding Brew|PASS||
|383700|Gai Plin's Imperial Brew|PASS||
|383707|Stormstout's Last Keg|PASS||
|383714|Training of Niuzao|PASS||
|383785|Counterstrike|PASS||
|383994|Dragonfire Brew|PASS||
|386937|Anvil and Stave|PASS||
|386965|Charred Passions|PASS||
|387046|Elusive Footwork|PASS||
|387219|Walk with the Ox|PASS||
|387230|Fluidity of Motion|PASS||
|387276|Strength of Spirit|PASS||
|387625|Staggering Strikes|PASS||
|387638|Shadowboxing Treads|PASS||
|388505|Quick Sip|PASS||
|388664|Calming Presence|PASS||
|388674|Ferocity of Xuen|PASS||
|388681|Elusive Mists|PASS||
|388811|Grace of the Crane|PASS||
|388812|Vivacious Vivification|PASS||
|388813|Expeditious Fortification|PASS||
|388814|Ironshell Brew|PASS||
|388917|Fortifying Brew|PASS||
|389579|Save Them All|PASS||
|389942|Face Palm|PASS||
|392900|Vigorous Expulsion|PASS||
|392910|Profound Rebuttal|PASS||
|393357|Tranquil Spirit|PASS||
|393516|Pretense of Instability|PASS||
|394110|Escape from Reality|PASS||
|394123|Fatal Touch|PASS||
|400629|Spirit of the Ox|PASS||
|414131|Yu'lon's Grace|PASS||
|414132|Dance of the Wind|PASS||
|418359|Press the Advantage|PASS||
|434774|Transcendence: Linked Spirits|PASS||
|449582|Lighter Than Air|PASS||
|450154|Rushing Reflexes|PASS||
|450335|Crashing Momentum|PASS||
|450391|Chi Wave|PASS||
|450426|Chi Proficiency|PASS||
|450427|Martial Instincts|PASS||
|450432|Pressure Points|PASS||
|450448|Peace and Prosperity|PASS||
|450503|Quick Footed|PASS||
|450508|Aspect of Harmony|PASS||
|450514|Wind's Reach|PASS||
|450520|Bounding Agility|PASS||
|450529|Coalescence|PASS||
|450553|Jade Walk|PASS||
|450560|Healing Winds|PASS||
|450569|Flow of Chi|PASS||
|450595|Spirit's Essence|PASS||
|450615|Flurry Strikes|PASS||
|450622|Swift Art|PASS||
|450631|Energy Transfer|PASS||
|450638|Celestial Determination|PASS||
|450867|Purified Spirit|PASS||
|450870|Harmonic Gambit|PASS||
|450875|Manifestation|PASS||
|450889|Balanced Stratagem|PASS||
|450892|Endless Draught|PASS||
|450912|Path of Resurgence|PASS||
|450965|Way of a Thousand Strikes|PASS||
|450979|Pride of Pandaria|PASS||
|450982|High Impact|PASS||
|450986|Against All Odds|PASS||
|450987|Veteran's Eye|PASS||
|450988|One Versus Many|PASS||
|450989|Efficient Training|PASS||
|450990|Martial Precision|FAIL|a:aura:280|
|450991|Whirling Steel|PASS||
|450992|Predictive Training|PASS||
|450993|Vigilant Watch|PASS||
|451017|Clarity of Purpose|PASS||
|451024|Overwhelming Force|PASS||
|451029|Mantra of Tenacity|PASS||
|451036|Mantra of Purity|PASS||
|451041|Tiger's Vigor|PASS||
|451043|Roar from the Heavens|PASS||
|454483|August Blessing|PASS||
|454484|One With the Wind|PASS||
|455068|Ox Stance|PASS||
|455139|Elixir of Determination|PASS||
|462087|Brewmaster Monk|PASS||
|462088|Monk|FAIL|d:fixture:no_passive_aura|
|1229376|Single-Button Assistant|PASS||
|1241059|Celestial Infusion|PASS||
|1241097|Niuzao's Resolve|PASS||
|1241136|Zen State|PASS||
|1243287|Diffuse Magic|PASS||
|1256978|Brewmaster Monk|FAIL|a:aura:647|
|1258153|Brewmaster Monk|PASS||
|1261543|Fast Feet|PASS||
|1262017|Fuel on the Fire|PASS||
|1262329|Empty the Cellar|PASS||
|1262334|Jade Flash|PASS||
|1262603|Stand Ready|PASS||
|1262610|Weapons of the Wall|PASS||
|1262612|Shado Over the Battlefield|PASS||
|1262628|Awakening Spirit|PASS||
|1262659|Vital Flame|PASS||
|1263245|Heart of the Ox|PASS||
|1263346|Swift as a Coursing River|PASS||
|1263353|Celestial Flames|PASS||
|1266719|Silent Sanctuary|PASS||
|1266733|Stillstep Coil|PASS||
|1266740|Reinvigoration|PASS||
|1270958|Harmonic Surge|PASS||
|1271048|Potential Energy|PASS||
|1271105|Meditative Focus|PASS||
|1272452|Chi Transfer|PASS||
|1272821|Wisdom of the Wall|PASS||
|1272844|Combat Stance|PASS||
|1272849|Initiator's Edge|PASS||

### Monk / Windwalker (269)

|Spell|Name|Result|Observed causes|
|---:|---|---|---|
|8647|Mystic Touch|PASS||
|100780|Tiger Palm|PASS||
|100784|Blackout Kick|PASS||
|101545|Flying Serpent Kick|FAIL|a:aura:373|
|101546|Spinning Crane Kick|PASS||
|101643|Transcendence|PASS||
|107270|Spinning Crane Kick|PASS||
|107428|Rising Sun Kick|PASS||
|107500|<DND> Monk Energy Driver|PASS||
|109132|Roll|PASS||
|113656|Fists of Fury|PASS||
|115008|Chi Torpedo|FAIL|a:aura:373|
|115078|Paralysis|PASS||
|115173|Celerity|PASS||
|115175|Soothing Mist|PASS||
|115178|Resuscitate|PASS||
|115396|Ascension|PASS||
|115546|Provoke|FAIL|d:fixture:provoke_self|
|115636|Mastery: Combo Strikes|PASS||
|116092|Afterlife|PASS||
|116095|Disable|PASS||
|116645|Teachings of the Monastery|PASS||
|116670|Vivify|PASS||
|116705|Spear Hand Strike|PASS||
|116812|Parry|PASS||
|116841|Tiger's Lust|PASS||
|116844|Ring of Peace|PASS||
|117418|Fists of Fury|PASS||
|117952|Crackling Jade Lightning|PASS||
|117959|Crackling Jade Lightning|PASS||
|119381|Leg Sweep|PASS||
|119611|Renewing Mist|PASS||
|119650|Energy Usage|PASS||
|120227|Leather Specialization|FAIL|d:fixture:no_passive_aura|
|121128|Touch of Death Notification Driver|PASS||
|121817|Combat Wisdom|PASS||
|122470|Touch of Karma|PASS||
|123586|Flying Serpent Kick|PASS||
|123904|Invoke Xuen, the White Tiger|PASS||
|124146|Dual Wield|PASS||
|124280|Touch of Karma|PASS||
|125355|Healing Sphere|PASS||
|125883|Zen Flight|FAIL|a:aura:206|
|126892|Zen Pilgrimage|PASS||
|126893|Zen Pilgrimage/Death Gate/Moonglade Storage Aura I|PASS||
|126895|Zen Pilgrimage: Return|PASS||
|128595|Combat Conditioning|PASS||
|130610|Monk|PASS||
|130654|Chi Burst|PASS||
|132463|Chi Wave|PASS||
|132467|Chi Wave|PASS||
|137022|Monk|PASS||
|137025|Windwalker Monk|PASS||
|137384|Combo Breaker|PASS||
|152175|Whirling Dragon Punch|FAIL|d:fixture:InvalidTarget:SPELL_FAILED_CASTER_AURASTATE|
|157361|Roll Speed Controls|PASS||
|157411|Windwalking|PASS||
|157445|Critical Strikes|PASS||
|162697|Stat Negation Aura - Agility DPS|PASS||
|169340|Touch of Fatality|PASS||
|196740|Hit Combo|PASS||
|198898|Song of Chi-Ji|PASS||
|218164|Detox|PASS||
|226178|missing|FAIL|d:missing_store|
|231018|missing|FAIL|d:missing_store|
|261916|Blackout Kick|PASS||
|261917|Sharp Reflexes|PASS||
|264348|Tiger Tail Sweep|PASS||
|280197|Spiritual Focus|PASS||
|322101|Expel Harm|PASS||
|322109|Touch of Death|FAIL|d:fixture:healthy_touch_of_death_target|
|322113|Improved Touch of Death|PASS||
|322719|Afterlife|PASS||
|323999|Empowered Tiger Lightning|PASS||
|325201|Dance of Chi-Ji|PASS||
|325215|Touch of Death|PASS||
|328670|Hasty Provocation|PASS||
|343730|Spinning Crane Kick|PASS||
|343731|Disable|PASS||
|344359|Ancient Arts|PASS||
|346104|Windwalker Monk Two-Hand Adjustment|PASS||
|387276|Strength of Spirit|PASS||
|388664|Calming Presence|PASS||
|388674|Ferocity of Xuen|PASS||
|388681|Elusive Mists|PASS||
|388809|Fast Feet|PASS||
|388811|Grace of the Crane|PASS||
|388812|Vivacious Vivification|PASS||
|388813|Expeditious Fortification|PASS||
|388814|Ironshell Brew|PASS||
|388848|Crane Vortex|PASS||
|388849|Rising Star|PASS||
|388856|Touch of the Tiger|PASS||
|388917|Fortifying Brew|PASS||
|389579|Save Them All|PASS||
|391330|Meridian Strikes|PASS||
|391370|Drinking Horn Cover|PASS||
|391383|Hardened Soles|PASS||
|392900|Vigorous Expulsion|PASS||
|392910|Profound Rebuttal|PASS||
|392958|Glory of the Dawn|PASS||
|392979|Jade Ignition|PASS||
|392982|Shadowboxing Treads|PASS||
|392983|Strike of the Windlord|PASS||
|392985|Thunderfist|PASS||
|392986|Xuen's Bond|PASS||
|392993|Xuen's Battlegear|PASS||
|392994|Path of Jade|PASS||
|394110|Escape from Reality|PASS||
|394123|Fatal Touch|PASS||
|397768|Inner Peace|PASS||
|414131|Yu'lon's Grace|PASS||
|432181|Dance of the Wind|PASS||
|434774|Transcendence: Linked Spirits|PASS||
|442687|Xuen's Guidance|PASS||
|442719|Restore Balance|PASS||
|442743|Temple Training|PASS||
|442747|Niuzao's Protection|PASS||
|443059|Jade Sanctuary|PASS||
|443087|Courage of the White Tiger|PASS||
|443110|Strength of the Black Ox|PASS||
|443294|Heart of the Jade Serpent|PASS||
|443566|Chi-Ji's Swiftness|PASS||
|443571|Inner Compass|PASS||
|443589|Unity Within|PASS||
|449582|Lighter Than Air|PASS||
|450154|Rushing Reflexes|PASS||
|450335|Crashing Momentum|PASS||
|450426|Chi Proficiency|PASS||
|450427|Martial Instincts|PASS||
|450432|Pressure Points|PASS||
|450448|Peace and Prosperity|PASS||
|450503|Quick Footed|PASS||
|450508|Aspect of Harmony|PASS||
|450514|Wind's Reach|PASS||
|450520|Bounding Agility|PASS||
|450529|Coalescence|PASS||
|450553|Jade Walk|PASS||
|450560|Healing Winds|PASS||
|450569|Flow of Chi|PASS||
|450595|Spirit's Essence|PASS||
|450615|Flurry Strikes|PASS||
|450622|Swift Art|PASS||
|450631|Energy Transfer|PASS||
|450638|Celestial Determination|PASS||
|450867|Purified Spirit|PASS||
|450870|Harmonic Gambit|PASS||
|450875|Manifestation|PASS||
|450889|Balanced Stratagem|PASS||
|450892|Endless Draught|PASS||
|450912|Path of Resurgence|PASS||
|450965|Way of a Thousand Strikes|PASS||
|450979|Pride of Pandaria|PASS||
|450982|High Impact|PASS||
|450986|Against All Odds|PASS||
|450987|Veteran's Eye|PASS||
|450988|One Versus Many|PASS||
|450989|Efficient Training|PASS||
|450990|Martial Precision|FAIL|a:aura:280|
|450991|Whirling Steel|PASS||
|450992|Predictive Training|PASS||
|450993|Vigilant Watch|PASS||
|451017|Clarity of Purpose|PASS||
|451024|Overwhelming Force|PASS||
|451029|Mantra of Tenacity|PASS||
|451036|Mantra of Purity|PASS||
|451041|Tiger's Vigor|PASS||
|451043|Roar from the Heavens|PASS||
|451294|Momentum Boost|PASS||
|451485|Brawler's Intensity|PASS||
|451498|Energy Burst|PASS||
|451515|Sequenced Strikes|PASS||
|451524|Revolving Whirl|PASS||
|451529|Knowledge of the Broken Temple|PASS||
|451573|Singularly Focused Jade|PASS||
|451576|Communion With Wind|PASS||
|451823|Dual Threat|PASS||
|452137|Flurry of Xuen|PASS||
|454969|Memory of the Monastery|PASS||
|457974|Jadefire Stomp|PASS||
|458623|Ferociousness|PASS||
|462088|Monk|FAIL|d:fixture:no_passive_aura|
|462091|Windwalker Monk|PASS||
|1217413|Slicing Winds|PASS||
|1222923|Windwalker Monk|FAIL|a:aura:647; a:aura:649|
|1229376|Single-Button Assistant|PASS||
|1243287|Diffuse Magic|PASS||
|1248704|Skyfire Heel|PASS||
|1248747|Crashing Fists|PASS||
|1248749|Cyclone's Drift|PASS||
|1248833|Airborne Rhythm|PASS||
|1248837|Hurricane's Vault|PASS||
|1248989|Celestial Conduit|PASS||
|1249625|Zenith|PASS||
|1249832|Obsidian Spiral|PASS||
|1250041|Harmonic Combo|PASS||
|1250042|Echo Technique|PASS||
|1250566|Rushing Wind Kick|PASS||
|1258122|Windwalker Monk|PASS||
|1262603|Stand Ready|PASS||
|1262610|Weapons of the Wall|PASS||
|1262612|Shado Over the Battlefield|PASS||
|1262667|Yu'lon's Avatar|PASS||
|1262672|Flowing Wisdom|PASS||
|1266719|Silent Sanctuary|PASS||
|1266733|Stillstep Coil|PASS||
|1266740|Reinvigoration|PASS||
|1270958|Harmonic Surge|PASS||
|1271048|Potential Energy|PASS||
|1271105|Meditative Focus|PASS||
|1272175|Universal Energy|PASS||
|1272415|Sunfire Spiral|PASS||
|1272452|Chi Transfer|PASS||
|1272678|Weapon of Wind|PASS||
|1272685|Martial Agility|PASS||
|1272694|Zenith Stomp|PASS||
|1272781|Tiger Fang|PASS||
|1272821|Wisdom of the Wall|PASS||
|1272844|Combat Stance|PASS||
|1272849|Initiator's Edge|PASS||
|1273154|Path of the Falling Star|PASS||

### Monk / Mistweaver (270)

|Spell|Name|Result|Observed causes|
|---:|---|---|---|
|8647|Mystic Touch|PASS||
|100780|Tiger Palm|PASS||
|100784|Blackout Kick|PASS||
|101546|Spinning Crane Kick|PASS||
|101643|Transcendence|PASS||
|107270|Spinning Crane Kick|PASS||
|107428|Rising Sun Kick|PASS||
|107500|<DND> Monk Energy Driver|PASS||
|109132|Roll|PASS||
|115008|Chi Torpedo|FAIL|a:aura:373|
|115078|Paralysis|PASS||
|115151|Renewing Mist|PASS||
|115173|Celerity|PASS||
|115175|Soothing Mist|PASS||
|115178|Resuscitate|PASS||
|115310|Revival|PASS||
|115313|Summon Jade Serpent Statue|PASS||
|115450|Detox|PASS||
|115546|Provoke|FAIL|d:fixture:provoke_self|
|115869|Mana Tea|PASS||
|116095|Disable|PASS||
|116645|Teachings of the Monastery|PASS||
|116670|Vivify|PASS||
|116680|Thunder Focus Tea|PASS||
|116812|Parry|PASS||
|116841|Tiger's Lust|PASS||
|116844|Ring of Peace|PASS||
|116849|Life Cocoon|FAIL|a:aura:259|
|117418|Fists of Fury|PASS||
|117907|Mastery: Gust of Mists|PASS||
|117952|Crackling Jade Lightning|PASS||
|117959|Crackling Jade Lightning|PASS||
|119381|Leg Sweep|PASS||
|119611|Renewing Mist|PASS||
|119650|Energy Usage|PASS||
|120224|Leather Specialization|FAIL|d:fixture:no_passive_aura|
|121128|Touch of Death Notification Driver|PASS||
|122280|Healing Elixir|PASS||
|123586|Flying Serpent Kick|PASS||
|124280|Touch of Karma|PASS||
|124682|Enveloping Mist|PASS||
|125355|Healing Sphere|PASS||
|125883|Zen Flight|FAIL|a:aura:206|
|126892|Zen Pilgrimage|PASS||
|126893|Zen Pilgrimage/Death Gate/Moonglade Storage Aura I|PASS||
|126895|Zen Pilgrimage: Return|PASS||
|130610|Monk|PASS||
|130654|Chi Burst|PASS||
|132463|Chi Wave|PASS||
|132467|Chi Wave|PASS||
|137022|Monk|PASS||
|137024|Mistweaver Monk|PASS||
|157361|Roll Speed Controls|PASS||
|157411|Windwalking|PASS||
|162701|Stat Negation Aura - Intellect Healer|PASS||
|169340|Touch of Fatality|PASS||
|173841|Pool of Mists|PASS||
|197895|Focused Thunder|PASS||
|197900|Mist Wrap|PASS||
|197915|Lifecycles|PASS||
|198898|Song of Chi-Ji|PASS||
|202424|Chrysalis|PASS||
|202577|Dome of Mist|PASS||
|212051|Reawaken|PASS||
|226178|missing|FAIL|d:missing_store|
|231018|missing|FAIL|d:missing_store|
|264348|Tiger Tail Sweep|PASS||
|274586|Invigorating Mists|PASS||
|274909|Rising Mist|PASS||
|322101|Expel Harm|PASS||
|322109|Touch of Death|FAIL|d:fixture:healthy_touch_of_death_target|
|322113|Improved Touch of Death|PASS||
|322118|Invoke Yu'lon, the Jade Serpent|PASS||
|325197|Invoke Chi-Ji, the Red Crane|PASS||
|328670|Hasty Provocation|PASS||
|343655|Celestial Harmony|PASS||
|344359|Ancient Arts|PASS||
|387991|Tear of Morning|PASS||
|388020|Resplendent Mist|PASS||
|388023|Ancient Teachings|PASS||
|388031|Jade Bond|PASS||
|388038|Yu'lon's Whisper|PASS||
|388212|Gift of the Celestials|PASS||
|388218|Calming Coalescence|PASS||
|388491|Secret Infusion|FAIL|a:aura:285|
|388511|Overflowing Mists|FAIL|a:aura:285|
|388548|Mists of Life|PASS||
|388551|Uplifted Spirits|PASS||
|388593|Peaceful Mending|PASS||
|388615|Restoral|PASS||
|388664|Calming Presence|PASS||
|388674|Ferocity of Xuen|PASS||
|388681|Elusive Mists|PASS||
|388682|Misty Peaks|PASS||
|388701|Dancing Mists|PASS||
|388779|Way of the Crane|PASS||
|388809|Fast Feet|PASS||
|388811|Grace of the Crane|PASS||
|388812|Vivacious Vivification|PASS||
|388813|Expeditious Fortification|PASS||
|388814|Ironshell Brew|PASS||
|388847|Rapid Diffusion|PASS||
|388874|Improved Detox|PASS||
|388917|Fortifying Brew|PASS||
|389579|Save Them All|PASS||
|394110|Escape from Reality|PASS||
|394123|Fatal Touch|PASS||
|399226|Burst of Life|PASS||
|399491|Sheilun's Gift|PASS||
|400053|Veil of Pride|PASS||
|404408|Legacy of Wisdom|PASS||
|414131|Yu'lon's Grace|PASS||
|422031|Energizing Brew|PASS||
|428200|Mistweaver Monk|FAIL|a:aura:647; a:aura:648; a:aura:649|
|432181|Dance of the Wind|PASS||
|434774|Transcendence: Linked Spirits|PASS||
|438439|Dance of Chi-Ji|PASS||
|440008|Peer Into Peace|PASS||
|442687|Xuen's Guidance|PASS||
|442719|Restore Balance|PASS||
|442743|Temple Training|PASS||
|442747|Niuzao's Protection|PASS||
|443028|Celestial Conduit|PASS||
|443059|Jade Sanctuary|PASS||
|443087|Courage of the White Tiger|PASS||
|443110|Strength of the Black Ox|PASS||
|443294|Heart of the Jade Serpent|PASS||
|443566|Chi-Ji's Swiftness|PASS||
|443571|Inner Compass|PASS||
|443589|Unity Within|PASS||
|443625|Yu'lon's Knowledge|PASS||
|446260|Crane Style|PASS||
|446326|Zen Pulse|PASS||
|446345|Deep Clarity|PASS||
|449582|Lighter Than Air|PASS||
|450154|Rushing Reflexes|PASS||
|450335|Crashing Momentum|PASS||
|450391|Chi Wave|PASS||
|450426|Chi Proficiency|PASS||
|450427|Martial Instincts|PASS||
|450432|Pressure Points|PASS||
|450448|Peace and Prosperity|PASS||
|450503|Quick Footed|PASS||
|450508|Aspect of Harmony|PASS||
|450514|Wind's Reach|PASS||
|450520|Bounding Agility|PASS||
|450529|Coalescence|PASS||
|450553|Jade Walk|PASS||
|450560|Healing Winds|PASS||
|450569|Flow of Chi|PASS||
|450595|Spirit's Essence|PASS||
|450615|Flurry Strikes|PASS||
|450622|Swift Art|PASS||
|450638|Celestial Determination|PASS||
|450867|Purified Spirit|PASS||
|450870|Harmonic Gambit|PASS||
|450875|Manifestation|PASS||
|450889|Balanced Stratagem|PASS||
|450892|Endless Draught|PASS||
|450912|Path of Resurgence|PASS||
|450965|Way of a Thousand Strikes|PASS||
|450979|Pride of Pandaria|PASS||
|450982|High Impact|PASS||
|450986|Against All Odds|PASS||
|450987|Veteran's Eye|PASS||
|450988|One Versus Many|PASS||
|450989|Efficient Training|PASS||
|450990|Martial Precision|FAIL|a:aura:280|
|450991|Whirling Steel|PASS||
|450992|Predictive Training|PASS||
|450993|Vigilant Watch|PASS||
|451017|Clarity of Purpose|PASS||
|451024|Overwhelming Force|PASS||
|451029|Mantra of Tenacity|PASS||
|451036|Mantra of Purity|PASS||
|451041|Tiger's Vigor|PASS||
|451043|Roar from the Heavens|PASS||
|458431|Lotus Infusion|PASS||
|462088|Monk|FAIL|d:fixture:no_passive_aura|
|462090|Mistweaver Monk|FAIL|a:aura:320|
|467270|Refreshment|PASS||
|467293|Jadefire Teachings|PASS||
|467307|Rushing Wind Kick|PASS||
|467316|Jade Empowerment|PASS||
|471761|Emperor's Favor|PASS||
|1229376|Single-Button Assistant|PASS||
|1242910|Jade Infusion|PASS||
|1243155|Way of the Serpent|PASS||
|1243287|Diffuse Magic|PASS||
|1258138|Mistweaver Monk|FAIL|a:aura:648|
|1262603|Stand Ready|PASS||
|1262610|Weapons of the Wall|PASS||
|1262612|Shado Over the Battlefield|PASS||
|1262667|Yu'lon's Avatar|PASS||
|1262672|Flowing Wisdom|PASS||
|1262756|Stampede of the Ancients|PASS||
|1266719|Silent Sanctuary|PASS||
|1266733|Stillstep Coil|PASS||
|1266734|Serene Surge|PASS||
|1266740|Reinvigoration|PASS||
|1266748|Vital Clarity|FAIL|a:aura:308|
|1266811|Mist Caller|PASS||
|1268807|Emperor's Elixir|PASS||
|1268817|Misty Coalescence|PASS||
|1270621|Tranquil Tea|PASS||
|1270958|Harmonic Surge|PASS||
|1271048|Potential Energy|PASS||
|1271105|Meditative Focus|PASS||
|1271431|Amplified Rush|PASS||
|1272452|Chi Transfer|PASS||
|1272821|Wisdom of the Wall|PASS||
|1272844|Combat Stance|PASS||
|1272849|Initiator's Edge|PASS||
|1273154|Path of the Falling Star|PASS||
|1277302|Morning Breeze|PASS||
|1277444|Chi Warding|PASS||
|1280297|Mistline|PASS||
|1285660|Serene Vitality|PASS||
|1299572|Vital Expenditure|PASS||

### Demon Hunter / Havoc (577)

|Spell|Name|Result|Observed causes|
|---:|---|---|---|
|131347|Glide|FAIL|a:aura:409|
|162243|Demon's Bite|FAIL|d:fixture:InvalidTarget:SPELL_FAILED_CASTER_AURASTATE|
|162697|Stat Negation Aura - Agility DPS|PASS||
|162794|Chaos Strike|PASS||
|178940|Shattered Souls|PASS||
|178976|Leather Specialization|FAIL|d:fixture:no_passive_aura|
|179057|Chaos Nova|PASS||
|183752|Disrupt|PASS||
|183782|Disrupting Fury|PASS||
|185123|Throw Glaive|PASS||
|185164|Mastery: Demonic Presence|PASS||
|185245|Torment|PASS||
|188499|Blade Dance|FAIL|a:aura:483|
|188501|Spectral Sight|FAIL|a:effect:167|
|191427|Metamorphosis|FAIL|a:aura:443|
|195072|Fel Rush|PASS||
|196055|Double Jump|PASS||
|196718|Darkness|PASS||
|197125|Chaos Strike|PASS||
|197130|Glide|FAIL|a:aura:406|
|198013|Eye Beam|FAIL|d:fixture:logger_external_consumer:85|
|198589|Blur|PASS||
|198793|Vengeful Retreat|PASS||
|203550|Blind Fury|PASS||
|203555|Demon Blades|PASS||
|203724|Parry|PASS||
|204909|Soul Rending|FAIL|a:aura:443|
|205411|Desperate Instincts|PASS||
|206416|First Blood|PASS||
|206476|Exergy|PASS||
|206478|Demonic Appetite|PASS||
|207347|Aura of Pain|PASS||
|207684|Sigil of Misery|PASS||
|212611|Demon Hunter|PASS||
|212612|Havoc Demon Hunter|PASS||
|213010|Charred Warblades|PASS||
|213410|Demonic|PASS||
|217832|Imprison|PASS||
|221351|Critical Strikes|PASS||
|226178|missing|FAIL|d:missing_store|
|231018|missing|FAIL|d:missing_store|
|232893|Felblade|PASS||
|255256|missing|FAIL|d:missing_store|
|255260|Chaos Brand|PASS||
|257191|missing|FAIL|d:missing_store|
|258860|Essence Break|PASS||
|258881|Trail of Ruin|PASS||
|258887|Cycle of Hatred|PASS||
|258920|Immolation Aura|FAIL|a:aura:220|
|278326|Consume Magic|PASS||
|278386|Demonic Wards|PASS||
|320313|Swallowed Anger|PASS||
|320331|Infernal Armor|PASS||
|320361|Improved Disrupt|PASS||
|320364|Immolation Aura|PASS||
|320374|Burning Hatred|PASS||
|320377|Immolation Aura|PASS||
|320386|Bouncing Glaives|PASS||
|320402|Blade Dance|PASS||
|320413|Critical Chaos|PASS||
|320416|Blazing Path|PASS||
|320418|Improved Sigil of Misery|PASS||
|320635|Vengeful Bonds|PASS||
|320654|Pursuit|PASS||
|320770|Unrestrained Fury|PASS||
|328725|Mortal Dance|PASS||
|343206|Improved Chaos Strike|PASS||
|343311|Furious Gaze|PASS||
|344859|Demon's Bite|PASS||
|344862|Chaos Strike|PASS||
|344865|Fel Rush|PASS||
|347461|Unbound Chaos|PASS||
|370965|The Hunt|PASS||
|388106|Soulscar|PASS||
|388107|Ragefire|PASS||
|388108|Initiative|PASS||
|388112|Chaotic Transformation|PASS||
|388113|Isolated Prey|PASS||
|388116|Shattered Destiny|PASS||
|388118|Know Your Enemy|PASS||
|389687|Chaos Theory|PASS||
|389688|Tactical Retreat|PASS||
|389693|Inner Demon|PASS||
|389695|Will of the Illidari|PASS||
|389696|Illidari Knowledge|PASS||
|389763|Master of the Glaive|PASS||
|389781|Long Night|PASS||
|389783|Pitch Black|PASS||
|389824|Shattered Restoration|PASS||
|389846|Felfire Haste|PASS||
|389849|Lost in Darkness|PASS||
|389977|Relentless Onslaught|PASS||
|389978|Dancing with Fate|PASS||
|390152|Collective Anguish|PASS||
|390154|Serrated Glaive|PASS||
|390158|Growing Inferno|PASS||
|391189|Burning Wound|PASS||
|391275|Accelerated Blade|PASS||
|391397|Erratic Felheart|PASS||
|391409|Aldrachi Design|PASS||
|393029|Furious Throws|PASS||
|393822|Internal Struggle|PASS||
|427640|Inertia|PASS||
|427775|A Fire Inside|FAIL|a:aura:220; a:aura:285|
|427776|Deflecting Dance|PASS||
|427794|Never Say Die|FAIL|a:aura:443|
|428232|Scars of Suffering|PASS||
|428241|Demon Hide|PASS||
|428492|Chaotic Disposition|PASS||
|428607|Live by the Glaive|PASS||
|429211|Champion of the Glaive|PASS||
|442290|Art of the Glaive|PASS||
|442492|Incisive Blade|PASS||
|442497|Keen Engagement|PASS||
|442502|Warblade's Hunger|PASS||
|442679|Reaver's Mark|PASS||
|442683|Aldrachi Tactics|PASS||
|442686|Thrill of the Fight|PASS||
|442714|Army Unto Oneself|PASS||
|442718|Fury of the Aldrachi|PASS||
|442736|Incorruptible Spirit|PASS||
|442806|Wounded Quarry|PASS||
|444926|Evasive Action|PASS||
|444931|Unhindered Assault|PASS||
|444997|Preemptive Strike|PASS||
|452402|Demonsurge|PASS||
|452403|Wave of Debilitation|PASS||
|452404|Pursuit of Angriness|PASS||
|452405|Focused Hatred|PASS||
|452406|Set Fire to the Pain|PASS||
|452407|Improved Soul Rending|PASS||
|452408|Burning Blades|PASS||
|452409|Violent Transformation|PASS||
|452410|Enduring Torment|PASS||
|452411|Untethered Fury|PASS||
|452412|Student of Suffering|PASS||
|452413|Flamebound|PASS||
|452414|Monster Rising|PASS||
|452415|Demonic Intensity|PASS||
|462065|Demon Hunter|FAIL|a:aura:320|
|462066|Havoc Demon Hunter|PASS||
|1220506|Screaming Brutality|PASS||
|1229376|Single-Button Assistant|PASS||
|1244557|Glaive Tempest|PASS||
|1253304|Voidfall|PASS||
|1253377|Meteoric Rise|PASS||
|1253391|Meteoric Fall|PASS||
|1253399|Path to Oblivion|PASS||
|1253402|State of Matter|PASS||
|1253415|Celestial Echoes|PASS||
|1253668|Swift Erasure|PASS||
|1253676|Doomsayer|PASS||
|1253769|Catastrophe|PASS||
|1253805|Final Hour|PASS||
|1253817|Otherworldly Focus|PASS||
|1256245|Phase Shift|PASS||
|1256295|Mass Acceleration|PASS||
|1256307|Dark Matter|PASS||
|1256353|World Killer|PASS||
|1256955|Havoc Demon Hunter|FAIL|a:aura:646; a:aura:647; a:aura:649|
|1266296|Focused Ire|PASS||
|1266307|Demonic Resilience|PASS||
|1266316|Burn It Out|PASS||
|1266326|Furious|PASS||
|1266328|Remorseless|PASS||
|1266329|Demon Muzzle|PASS||
|1266330|Soul Splitter|PASS||
|1266493|Wings of Wrath|PASS||
|1266496|Soul Cleanse|PASS||
|1266497|First In, Last Out|PASS||
|1266500|Final Breath|PASS||
|1266762|Felbound|PASS||
|1272138|Keen Edge|PASS||
|1272143|Broken Spirit|PASS||
|1272153|Bladecraft|PASS||
|1272364|Blind Focus|PASS||
|1272405|Undying Embers|PASS||
|1272453|Volatile Instinct|PASS||
|1279247|Harness the Cosmos|PASS||

### Demon Hunter / Vengeance (581)

|Spell|Name|Result|Observed causes|
|---:|---|---|---|
|131347|Glide|FAIL|a:aura:409|
|162700|Stat Negation Aura - Agility Tank|PASS||
|179057|Chaos Nova|PASS||
|183752|Disrupt|PASS||
|183782|Disrupting Fury|PASS||
|185123|Throw Glaive|PASS||
|185245|Torment|PASS||
|187827|Metamorphosis|FAIL|a:aura:443|
|188501|Spectral Sight|FAIL|a:effect:167|
|189110|Infernal Strike|PASS||
|189926|Increased Threat|PASS||
|191427|Metamorphosis|FAIL|a:aura:443|
|195761|missing|FAIL|d:missing_store|
|196055|Double Jump|PASS||
|196718|Darkness|PASS||
|197130|Glide|FAIL|a:aura:406|
|198793|Vengeful Retreat|PASS||
|202137|Sigil of Silence|PASS||
|202138|Sigil of Chains|PASS||
|203513|Demonic Wards|PASS||
|203720|Demon Spikes|PASS||
|203724|Parry|PASS||
|203747|Mastery: Fel Blood|PASS||
|203783|Shear|PASS||
|204021|Fiery Brand|PASS||
|204157|Throw Glaive|PASS||
|204254|Shattered Souls|PASS||
|204596|Sigil of Flame|PASS||
|204909|Soul Rending|FAIL|a:aura:443|
|207197|Riposte|PASS||
|207347|Aura of Pain|PASS||
|207387|Painbringer|PASS||
|207407|Soul Carver|PASS||
|207548|Agonizing Flames|PASS||
|207684|Sigil of Misery|PASS||
|207697|Feast of Souls|PASS||
|207739|Burning Alive|PASS||
|209258|Last Resort|PASS||
|209281|Quickened Sigils|PASS||
|212084|Fel Devastation|PASS||
|212611|Demon Hunter|PASS||
|212613|Vengeance Demon Hunter|PASS||
|213010|Charred Warblades|PASS||
|217832|Imprison|PASS||
|218612|Feed the Demon|PASS||
|221351|Critical Strikes|PASS||
|226178|missing|FAIL|d:missing_store|
|226359|Leather Specialization|FAIL|d:fixture:no_passive_aura|
|227174|Fallout|PASS||
|228477|Soul Cleave|PASS||
|231018|missing|FAIL|d:missing_store|
|232893|Felblade|PASS||
|247454|Spirit Bomb|FAIL|d:fixture:InvalidTarget:SPELL_FAILED_CASTER_AURASTATE|
|255256|missing|FAIL|d:missing_store|
|255260|Chaos Brand|PASS||
|258920|Immolation Aura|FAIL|a:aura:220|
|263642|Fracture|PASS||
|268175|Void Reaver|PASS||
|278326|Consume Magic|PASS||
|320313|Swallowed Anger|PASS||
|320331|Infernal Armor|PASS||
|320361|Improved Disrupt|PASS||
|320364|Immolation Aura|PASS||
|320378|Immolation Aura|PASS||
|320380|Thick Skin|PASS||
|320386|Bouncing Glaives|PASS||
|320387|Perfectly Balanced Glaive|PASS||
|320416|Blazing Path|PASS||
|320418|Improved Sigil of Misery|PASS||
|320635|Vengeful Bonds|PASS||
|320639|Fel Devastation|PASS||
|320654|Pursuit|PASS||
|320770|Unrestrained Fury|PASS||
|320794|Sigil of Flame|PASS||
|320962|Fiery Brand|PASS||
|321028|Demon Spikes|PASS||
|321067|Metamorphosis|PASS||
|321068|Metamorphosis|PASS||
|321299|Mastery: Fel Blood|PASS||
|326853|Ruinous Bulwark|PASS||
|336639|Charred Flesh|PASS||
|343014|Revel in Pain|PASS||
|343207|Focused Cleave|PASS||
|344859|Demon's Bite|PASS||
|344862|Chaos Strike|PASS||
|344865|Fel Rush|PASS||
|389220|Fiery Demise|PASS||
|389695|Will of the Illidari|PASS||
|389696|Illidari Knowledge|PASS||
|389705|Fel Flame Fortification|PASS||
|389708|Darkglare Boon|PASS||
|389711|Soulmonger|PASS||
|389715|Chains of Anger|PASS||
|389718|Cycle of Binding|PASS||
|389720|Calcified Spikes|PASS||
|389724|Felfire Fist|PASS||
|389729|Retaliation|PASS||
|389732|Down in Flames|PASS||
|389763|Master of the Glaive|PASS||
|389781|Long Night|PASS||
|389783|Pitch Black|PASS||
|389824|Shattered Restoration|PASS||
|389846|Felfire Haste|PASS||
|389849|Lost in Darkness|PASS||
|389958|Frailty|PASS||
|389976|Vulnerability|PASS||
|389985|Soulcrush|PASS||
|390163|Sigil of Spite|PASS||
|390213|Burning Blood|PASS||
|390808|Volatile Flameblood|PASS||
|391178|Roaring Fire|PASS||
|391397|Erratic Felheart|PASS||
|391409|Aldrachi Design|PASS||
|393822|Internal Struggle|PASS||
|393827|Stoke the Flames|PASS||
|395446|Soul Sigils|PASS||
|428603|Ascending Flame|PASS||
|428607|Live by the Glaive|PASS||
|429211|Champion of the Glaive|PASS||
|442290|Art of the Glaive|PASS||
|442492|Incisive Blade|PASS||
|442497|Keen Engagement|PASS||
|442502|Warblade's Hunger|PASS||
|442679|Reaver's Mark|PASS||
|442683|Aldrachi Tactics|PASS||
|442686|Thrill of the Fight|PASS||
|442714|Army Unto Oneself|PASS||
|442718|Fury of the Aldrachi|PASS||
|442736|Incorruptible Spirit|PASS||
|442806|Wounded Quarry|PASS||
|444926|Evasive Action|PASS||
|444931|Unhindered Assault|PASS||
|444997|Preemptive Strike|PASS||
|452402|Demonsurge|PASS||
|452403|Wave of Debilitation|PASS||
|452404|Pursuit of Angriness|PASS||
|452405|Focused Hatred|PASS||
|452406|Set Fire to the Pain|PASS||
|452407|Improved Soul Rending|PASS||
|452408|Burning Blades|PASS||
|452409|Violent Transformation|PASS||
|452410|Enduring Torment|PASS||
|452411|Untethered Fury|PASS||
|452412|Student of Suffering|PASS||
|452413|Flamebound|PASS||
|452414|Monster Rising|PASS||
|452415|Demonic Intensity|PASS||
|462065|Demon Hunter|FAIL|a:aura:320|
|462067|Vengeance Demon Hunter|PASS||
|1229376|Single-Button Assistant|PASS||
|1253304|Voidfall|PASS||
|1253377|Meteoric Rise|PASS||
|1253391|Meteoric Fall|PASS||
|1253399|Path to Oblivion|PASS||
|1253402|State of Matter|PASS||
|1253415|Celestial Echoes|PASS||
|1253668|Swift Erasure|PASS||
|1253676|Doomsayer|PASS||
|1253769|Catastrophe|PASS||
|1253805|Final Hour|PASS||
|1253817|Otherworldly Focus|PASS||
|1256245|Phase Shift|PASS||
|1256295|Mass Acceleration|PASS||
|1256307|Dark Matter|PASS||
|1256353|World Killer|PASS||
|1256959|Vengeance Demon Hunter|FAIL|a:aura:647|
|1265800|Tempered Steel|PASS||
|1265818|Vengeful Beast|PASS||
|1265924|Soul Barrier|PASS||
|1266296|Focused Ire|PASS||
|1266307|Demonic Resilience|PASS||
|1266316|Burn It Out|PASS||
|1266326|Furious|PASS||
|1266328|Remorseless|PASS||
|1266329|Demon Muzzle|PASS||
|1266330|Soul Splitter|PASS||
|1266493|Wings of Wrath|PASS||
|1266496|Soul Cleanse|PASS||
|1266497|First In, Last Out|PASS||
|1266500|Final Breath|PASS||
|1266762|Felbound|PASS||
|1272138|Keen Edge|PASS||
|1272143|Broken Spirit|PASS||
|1272153|Bladecraft|PASS||
|1272364|Blind Focus|PASS||
|1272405|Undying Embers|PASS||
|1272453|Volatile Instinct|PASS||
|1279247|Harness the Cosmos|PASS||

### Evoker / Devastation (1467)

|Spell|Name|Result|Observed causes|
|---:|---|---|---|
|351338|Quell|PASS||
|353167|Evoker|PASS||
|355913|Emerald Blossom|PASS||
|356809|Devastation Evoker|PASS||
|356995|Disintegrate|PASS||
|357208|Fire Breath|PASS||
|357210|Deep Breath|FAIL|d:fixture:missing_creature_immunities|
|357211|Pyre|PASS||
|358267|Hover|FAIL|a:aura:527|
|358385|Landslide|PASS||
|359073|Eternity Surge|PASS||
|360806|Sleep Walk|PASS||
|360995|Verdant Embrace|PASS||
|361227|Return|PASS||
|361469|Living Flame|PASS||
|362969|Azure Strike|PASS||
|362980|Mastery: Giantkiller|PASS||
|363916|Obsidian Scales|PASS||
|364342|Blessing of the Bronze|PASS||
|365261|Emerald Blossom|PASS||
|365585|Expunge|PASS||
|365933|Aerial Mastery|PASS||
|365937|Ruby Embers|PASS||
|366524|Mail Specialization|FAIL|d:fixture:no_passive_aura|
|368838|Heavy Wingbeats|PASS||
|368970|Tail Swipe|PASS||
|369089|Volatility|PASS||
|369459|Source of Magic|FAIL|d:fixture:self_with_exclude_caster|
|369846|Feed the Flames|PASS||
|369908|Power Nexus|PASS||
|369909|Protracted Talons|PASS||
|369913|Natural Convergence|PASS||
|369939|Leaping Flames|PASS||
|369990|Ancient Flame|PASS||
|370455|Charged Blast|PASS||
|370553|Tip the Scales|PASS||
|370665|Rescue|FAIL|d:fixture:self_with_exclude_caster|
|370781|Imminent Destruction|PASS||
|370819|Scorching Embers|PASS||
|370821|Scintillation|PASS||
|370837|Engulfing Blaze|PASS||
|370839|Power Swell|PASS||
|370845|Spellweaver's Dominance|PASS||
|370867|Iridescence|PASS||
|370886|Bountiful Bloom|PASS||
|370888|Twin Guardian|PASS||
|371032|Terror of the Skies|PASS||
|371034|Lay Waste|PASS||
|371038|Honed Aggression|PASS||
|371806|Recall|PASS||
|372048|Oppressing Roar|FAIL|a:aura:232|
|372469|Scarlet Adaptation|PASS||
|374227|Zephyr|PASS||
|374251|Cauterizing Flame|PASS||
|374346|Overawe|PASS||
|374348|Renewing Blaze|PASS||
|374968|Time Spiral|PASS||
|375087|Dragonrage|PASS||
|375406|Obsidian Bulwark|PASS||
|375443|Clobbering Sweep|PASS||
|375510|Blast Furnace|PASS||
|375517|Extended Flight|PASS||
|375520|Innate Magic|PASS||
|375528|Forger of Mountains|PASS||
|375542|Exuberance|PASS||
|375544|Inherent Resistance|PASS||
|375554|Enkindled|PASS||
|375556|Tailwind|PASS||
|375561|Lush Growth|PASS||
|375574|Foci of Life|PASS||
|375618|Arcane Intensity|PASS||
|375721|Azure Essence Burst|PASS||
|375722|Essence Attunement|PASS||
|375725|Heat Wave|PASS||
|375757|Eternity's Span|PASS||
|375777|Causality|PASS||
|375797|Animosity|PASS||
|375801|Burnout|PASS||
|376164|Instinctive Arcana|PASS||
|376166|Draconic Legacy|PASS||
|376872|Ruby Essence Burst|PASS||
|376888|Tyranny|PASS||
|376930|Attuned to the Dream|PASS||
|381773|Permeating Chill|PASS||
|386272|Titanic Wrath|PASS||
|386283|Catalyze|PASS||
|386348|Onyx Legacy|PASS||
|386405|Inner Radiance|PASS||
|387341|Walloping Blow|PASS||
|387761|Panacea|PASS||
|387787|Regenerative Magic|FAIL|a:aura:443|
|390386|Fury of the Aspects|PASS||
|396571|Tempered Scales|PASS||
|400006|Energizing Flame|PASS||
|403213|missing|FAIL|d:missing_store|
|406732|Spatial Paradox|PASS||
|410352|Stretch Time|PASS||
|411164|Event Horizon|PASS||
|411165|Eye of Infinity|PASS||
|411212|Font of Magic|PASS||
|418101|Potent Mana|PASS||
|429483|Warp|PASS||
|431442|Chrono Flame|PASS||
|431484|Instability Matrix|PASS||
|431615|Reverberations|PASS||
|431657|Primacy|PASS||
|431695|Temporal Burst|PASS||
|431715|Nozdormu Adept|PASS||
|431873|Temporality|PASS||
|431874|Double-time|PASS||
|431875|Afterimage|PASS||
|431984|Time Convergence|PASS||
|432004|Golden Opportunity|PASS||
|432008|Motes of Acceleration|PASS||
|433871|Maneuverability|PASS||
|434300|Bombardments|PASS||
|436335|Mass Disintegrate|PASS||
|441176|Melt Armor|PASS||
|441180|Hardened Scales|PASS||
|441181|Menacing Presence|PASS||
|441206|Wingleader|PASS||
|441212|Extended Battle|PASS||
|441219|Diverted Power|PASS||
|441245|Onslaught|PASS||
|441246|Unrelenting Siege|PASS||
|441253|Nimble Flyer|PASS||
|441257|Slipstream|PASS||
|441705|Might of the Black Dragonflight|PASS||
|444016|Enkindle|PASS||
|444020|Burning Adrenaline|PASS||
|444088|Consume Flame|PASS||
|444322|Lifecinders|PASS||
|444843|Conduit of Flame|FAIL|d:fixture:logger_external_consumer:183|
|444845|Expanded Lungs|PASS||
|444849|Trailblazer|PASS||
|445074|Shape of Flame|PASS||
|445625|Titanic Precision|PASS||
|445958|Draconic Instincts|PASS||
|462075|Evoker|FAIL|d:fixture:no_passive_aura|
|462076|Devastation Evoker|PASS||
|1218447|Fulminous Roar|PASS||
|1219723|Azure Celerity|PASS||
|1229376|Single-Button Assistant|PASS||
|1256943|Devastation Evoker|FAIL|a:aura:647; a:aura:648; a:aura:649|
|1260484|Chronoboon|PASS||
|1260647|Overclock|PASS||
|1260745|Command Squadron|PASS||
|1261448|Concentrated Power|PASS||
|1261452|Refined Essence|PASS||
|1264269|Legacy of the Lifebinder|PASS||
|1264321|Deep Exhalation|PASS||
|1264365|Ashes in Motion|PASS||
|1264378|Unravel|PASS||
|1265802|Shattering Stars|PASS||
|1265826|Star Salvo|PASS||
|1265867|Azure Sweep|PASS||
|1265979|Twin Flame|PASS||
|1265992|Fire Torrent|PASS||
|1265993|Essence Well|PASS||
|1266151|Strafing Run|PASS||
|1267206|Strike from Above|PASS||
|1286123|Devastation Evoker|PASS||
|1291522|Chronal Dynamo|PASS||

### Evoker / Preservation (1468)

|Spell|Name|Result|Observed causes|
|---:|---|---|---|
|353167|Evoker|PASS||
|355913|Emerald Blossom|PASS||
|355936|Dream Breath|FAIL|d:fixture:missing_creature_immunities|
|356810|Preservation Evoker|PASS||
|356995|Disintegrate|PASS||
|357170|Time Dilation|PASS||
|357208|Fire Breath|PASS||
|357210|Deep Breath|FAIL|d:fixture:missing_creature_immunities|
|358267|Hover|FAIL|a:aura:527|
|358385|Landslide|PASS||
|359793|Fluttering Seedlings|PASS||
|359816|Dream Flight|FAIL|d:fixture:missing_creature_immunities|
|360806|Sleep Walk|PASS||
|360823|Naturalize|PASS||
|360995|Verdant Embrace|PASS||
|361178|Mass Return|PASS||
|361227|Return|PASS||
|361469|Living Flame|PASS||
|362874|Temporal Compression|PASS||
|362969|Azure Strike|PASS||
|363510|Mastery: Life-Binder|PASS||
|363534|Rewind|PASS||
|363916|Obsidian Scales|PASS||
|364342|Blessing of the Bronze|PASS||
|364343|Echo|PASS||
|365262|Improved Emerald Blossom|PASS||
|365585|Expunge|PASS||
|365933|Aerial Mastery|PASS||
|366155|Reversion|FAIL|a:aura:504|
|366524|Mail Specialization|FAIL|d:fixture:no_passive_aura|
|368412|Time of Need|PASS||
|368838|Heavy Wingbeats|PASS||
|368970|Tail Swipe|PASS||
|369297|Essence Burst|PASS||
|369459|Source of Magic|FAIL|d:fixture:self_with_exclude_caster|
|369908|Power Nexus|PASS||
|369909|Protracted Talons|PASS||
|369913|Natural Convergence|PASS||
|369939|Leaping Flames|PASS||
|369990|Ancient Flame|PASS||
|370062|Field of Dreams|PASS||
|370537|Stasis|PASS||
|370553|Tip the Scales|PASS||
|370665|Rescue|FAIL|d:fixture:self_with_exclude_caster|
|370886|Bountiful Bloom|PASS||
|370888|Twin Guardian|PASS||
|371032|Terror of the Skies|PASS||
|371257|Renewing Breath|PASS||
|371426|Life-Giver's Flame|PASS||
|371806|Recall|PASS||
|372048|Oppressing Roar|FAIL|a:aura:232|
|372233|Energy Loop|PASS||
|372469|Scarlet Adaptation|PASS||
|372527|Time Lord|PASS||
|373270|Lifebind|PASS||
|373834|Call of Ysera|PASS||
|373861|Temporal Anomaly|PASS||
|374227|Zephyr|PASS||
|374251|Cauterizing Flame|PASS||
|374346|Overawe|PASS||
|374348|Renewing Blaze|PASS||
|374968|Time Spiral|PASS||
|375406|Obsidian Bulwark|PASS||
|375443|Clobbering Sweep|PASS||
|375510|Blast Furnace|PASS||
|375517|Extended Flight|PASS||
|375520|Innate Magic|PASS||
|375528|Forger of Mountains|PASS||
|375542|Exuberance|PASS||
|375544|Inherent Resistance|PASS||
|375554|Enkindled|PASS||
|375556|Tailwind|PASS||
|375561|Lush Growth|PASS||
|375574|Foci of Life|PASS||
|375722|Essence Attunement|PASS||
|375783|Font of Magic|PASS||
|376138|Empath|PASS||
|376150|Spiritual Clarity|PASS||
|376164|Instinctive Arcana|PASS||
|376166|Draconic Legacy|PASS||
|376179|Lifeforce Mender|PASS||
|376204|Just in Time|PASS||
|376207|Delay Harm|PASS||
|376237|Nozdormu's Teachings|PASS||
|376239|Grace Period|PASS||
|376240|Timeless Magic|PASS||
|376930|Attuned to the Dream|PASS||
|377099|Spark of Insight|PASS||
|377100|Exhilarating Burst|PASS||
|378196|Golden Hour|PASS||
|381773|Permeating Chill|PASS||
|381921|Ouroboros|PASS||
|381922|Temporal Artificer|PASS||
|385696|Flow State|PASS||
|386405|Inner Radiance|PASS||
|387341|Walloping Blow|PASS||
|387761|Panacea|PASS||
|387787|Regenerative Magic|FAIL|a:aura:443|
|390386|Fury of the Aspects|PASS||
|396571|Tempered Scales|PASS||
|400006|Energizing Flame|PASS||
|403213|missing|FAIL|d:missing_store|
|406732|Spatial Paradox|PASS||
|410352|Stretch Time|PASS||
|418101|Potent Mana|PASS||
|429483|Warp|PASS||
|431442|Chrono Flame|PASS||
|431484|Instability Matrix|PASS||
|431615|Reverberations|PASS||
|431657|Primacy|PASS||
|431695|Temporal Burst|PASS||
|431715|Nozdormu Adept|PASS||
|431873|Temporality|PASS||
|431874|Double-time|PASS||
|431875|Afterimage|PASS||
|431984|Time Convergence|PASS||
|432004|Golden Opportunity|PASS||
|432008|Motes of Acceleration|PASS||
|433871|Maneuverability|PASS||
|434300|Bombardments|PASS||
|441176|Melt Armor|PASS||
|441180|Hardened Scales|PASS||
|441181|Menacing Presence|PASS||
|441206|Wingleader|PASS||
|441212|Extended Battle|PASS||
|441219|Diverted Power|PASS||
|441245|Onslaught|PASS||
|441246|Unrelenting Siege|PASS||
|441253|Nimble Flyer|PASS||
|441257|Slipstream|PASS||
|441705|Might of the Black Dragonflight|PASS||
|443177|Lifespark|PASS||
|443264|Titan's Gift|PASS||
|444016|Enkindle|PASS||
|444020|Burning Adrenaline|PASS||
|444088|Consume Flame|PASS||
|444322|Lifecinders|PASS||
|444843|Conduit of Flame|FAIL|d:fixture:logger_external_consumer:183|
|444845|Expanded Lungs|PASS||
|444849|Trailblazer|PASS||
|445074|Shape of Flame|PASS||
|445625|Titanic Precision|PASS||
|445958|Draconic Instincts|PASS||
|462075|Evoker|FAIL|d:fixture:no_passive_aura|
|462078|Preservation Evoker|PASS||
|1218447|Fulminous Roar|PASS||
|1229376|Single-Button Assistant|PASS||
|1237978|Tempo Charged|PASS||
|1239581|Unshakable|PASS||
|1241669|Dream Simulacrum|PASS||
|1241704|Wings of Liberty|PASS||
|1242031|Twin Echoes|PASS||
|1242745|Inner Flame|PASS||
|1256944|Preservation Evoker|FAIL|a:aura:647|
|1260484|Chronoboon|PASS||
|1260647|Overclock|PASS||
|1260745|Command Squadron|PASS||
|1261448|Concentrated Power|PASS||
|1261452|Refined Essence|PASS||
|1264269|Legacy of the Lifebinder|PASS||
|1264321|Deep Exhalation|PASS||
|1264365|Ashes in Motion|PASS||
|1264378|Unravel|PASS||
|1265979|Twin Flame|PASS||
|1265992|Fire Torrent|PASS||
|1265993|Essence Well|PASS||
|1267206|Strike from Above|PASS||
|1291522|Chronal Dynamo|PASS||
|1291636|Temporal Barrier|PASS||

### Evoker / Augmentation (1473)

|Spell|Name|Result|Observed causes|
|---:|---|---|---|
|351338|Quell|PASS||
|353167|Evoker|PASS||
|355913|Emerald Blossom|PASS||
|356995|Disintegrate|PASS||
|357208|Fire Breath|PASS||
|357210|Deep Breath|FAIL|d:fixture:missing_creature_immunities|
|358267|Hover|FAIL|a:aura:527|
|358385|Landslide|PASS||
|360806|Sleep Walk|PASS||
|360827|Blistering Scales|FAIL|a:aura:22|
|360995|Verdant Embrace|PASS||
|361021|Sense Power|PASS||
|361227|Return|PASS||
|361469|Living Flame|PASS||
|362969|Azure Strike|PASS||
|363916|Obsidian Scales|PASS||
|364342|Blessing of the Bronze|PASS||
|365261|Emerald Blossom|PASS||
|365585|Expunge|PASS||
|365933|Aerial Mastery|PASS||
|366524|Mail Specialization|FAIL|d:fixture:no_passive_aura|
|368838|Heavy Wingbeats|PASS||
|368970|Tail Swipe|PASS||
|369459|Source of Magic|FAIL|d:fixture:self_with_exclude_caster|
|369908|Power Nexus|PASS||
|369909|Protracted Talons|PASS||
|369913|Natural Convergence|PASS||
|369939|Leaping Flames|PASS||
|369990|Ancient Flame|PASS||
|370553|Tip the Scales|PASS||
|370665|Rescue|FAIL|d:fixture:self_with_exclude_caster|
|370886|Bountiful Bloom|PASS||
|370888|Twin Guardian|PASS||
|371032|Terror of the Skies|PASS||
|371806|Recall|PASS||
|372048|Oppressing Roar|FAIL|a:aura:232|
|372469|Scarlet Adaptation|PASS||
|374227|Zephyr|PASS||
|374251|Cauterizing Flame|PASS||
|374346|Overawe|PASS||
|374348|Renewing Blaze|PASS||
|374968|Time Spiral|PASS||
|375406|Obsidian Bulwark|PASS||
|375443|Clobbering Sweep|PASS||
|375510|Blast Furnace|PASS||
|375517|Extended Flight|PASS||
|375520|Innate Magic|PASS||
|375528|Forger of Mountains|PASS||
|375542|Exuberance|PASS||
|375544|Inherent Resistance|PASS||
|375554|Enkindled|PASS||
|375556|Tailwind|PASS||
|375561|Lush Growth|PASS||
|375574|Foci of Life|PASS||
|375722|Essence Attunement|PASS||
|375796|Hoarded Power|PASS||
|376164|Instinctive Arcana|PASS||
|376166|Draconic Legacy|PASS||
|376930|Attuned to the Dream|PASS||
|381773|Permeating Chill|PASS||
|386405|Inner Radiance|PASS||
|387341|Walloping Blow|PASS||
|387761|Panacea|PASS||
|387787|Regenerative Magic|FAIL|a:aura:443|
|390386|Fury of the Aspects|PASS||
|395152|Ebon Might|FAIL|d:fixture:self_with_exclude_caster|
|395153|Sands of Time|PASS||
|395160|Eruption|PASS||
|396043|Close as Clutchmates|PASS||
|396186|Augmentation Evoker|PASS||
|396187|Essence Burst|PASS||
|396286|Upheaval|PASS||
|396571|Tempered Scales|PASS||
|400006|Energizing Flame|PASS||
|403208|Draconic Attunements|PASS||
|403213|missing|FAIL|d:missing_store|
|403631|Breath of Eons|FAIL|d:fixture:missing_creature_immunities|
|404195|Defy Fate|PASS||
|404977|Time Skip|PASS||
|406041|Nourishing Sands|PASS||
|406380|Mastery: Timewalker|PASS||
|406659|Ricocheting Pyroclast|PASS||
|406732|Spatial Paradox|PASS||
|406904|Volcanism|PASS||
|406907|Regenerative Chitin|PASS||
|407243|Aspects' Favor|PASS||
|407814|Pupil of Alexstrasza|PASS||
|407866|Plot the Future|PASS||
|407869|Anachronism|PASS||
|407876|Accretion|PASS||
|408002|Tectonic Locus|PASS||
|408004|Momentum Shift|PASS||
|408083|Font of Magic|PASS||
|408233|Bestow Weyrnstone|FAIL|d:fixture:self_with_exclude_caster|
|408775|Ignition Rush|PASS||
|409267|Motes of Possibility|PASS||
|409311|Prescience|FAIL|a:effect:202|
|409329|Reactive Hide|PASS||
|409676|Chrono Ward|PASS||
|410253|Perilous Fate|PASS||
|410260|Overlord|PASS||
|410261|Inferno's Blessing|PASS||
|410352|Stretch Time|PASS||
|410643|Molten Blood|PASS||
|410685|Symbiotic Bloom|PASS||
|410687|Prolong Life|PASS||
|410784|Echoing Strike|PASS||
|412710|Timelessness|PASS||
|412713|Interwoven Threads|PASS||
|412723|Tomorrow, Today|PASS||
|412733|Unyielding Domain|PASS||
|412774|Fate Mirror|PASS||
|414969|Dream of Spring|PASS||
|418101|Potent Mana|PASS||
|429483|Warp|PASS||
|431442|Chrono Flame|PASS||
|431484|Instability Matrix|PASS||
|431615|Reverberations|PASS||
|431657|Primacy|PASS||
|431695|Temporal Burst|PASS||
|431715|Nozdormu Adept|PASS||
|431873|Temporality|PASS||
|431874|Double-time|PASS||
|431875|Afterimage|PASS||
|431984|Time Convergence|PASS||
|432004|Golden Opportunity|PASS||
|432008|Motes of Acceleration|PASS||
|433871|Maneuverability|PASS||
|434300|Bombardments|PASS||
|438587|Mass Eruption|PASS||
|441176|Melt Armor|PASS||
|441180|Hardened Scales|PASS||
|441181|Menacing Presence|PASS||
|441206|Wingleader|PASS||
|441212|Extended Battle|PASS||
|441219|Diverted Power|PASS||
|441245|Onslaught|PASS||
|441246|Unrelenting Siege|PASS||
|441253|Nimble Flyer|PASS||
|441257|Slipstream|PASS||
|441705|Might of the Black Dragonflight|PASS||
|454983|Arcane Reach|PASS||
|459120|Rumbling Earth|PASS||
|459537|Imminent Destruction|PASS||
|462074|Augmentation Evoker|PASS||
|462075|Evoker|FAIL|d:fixture:no_passive_aura|
|1229376|Single-Button Assistant|PASS||
|1250914|Clairvoyant|PASS||
|1256942|Augmentation Evoker|FAIL|a:aura:647|
|1260484|Chronoboon|PASS||
|1260647|Overclock|PASS||
|1260745|Command Squadron|PASS||
|1261448|Concentrated Power|PASS||
|1261452|Refined Essence|PASS||
|1264378|Unravel|PASS||
|1267206|Strike from Above|PASS||
|1268881|Improved Defy Fate|PASS||
|1291457|Mighty Inferno|PASS||
|1291522|Chronal Dynamo|PASS||

### Demon Hunter / Devourer (1480)

|Spell|Name|Result|Observed causes|
|---:|---|---|---|
|131347|Glide|FAIL|a:aura:409|
|162699|Stat Negation Aura - Intellect DPS|PASS||
|183752|Disrupt|PASS||
|183782|Disrupting Fury|PASS||
|185123|Throw Glaive|PASS||
|185245|Torment|PASS||
|188501|Spectral Sight|FAIL|a:effect:167|
|191427|Metamorphosis|FAIL|a:aura:443|
|196055|Double Jump|PASS||
|196718|Darkness|PASS||
|197130|Glide|FAIL|a:aura:406|
|198589|Blur|PASS||
|198793|Vengeful Retreat|PASS||
|203724|Parry|PASS||
|204909|Soul Rending|FAIL|a:aura:443|
|207347|Aura of Pain|PASS||
|207684|Sigil of Misery|PASS||
|212611|Demon Hunter|PASS||
|213010|Charred Warblades|PASS||
|217832|Imprison|PASS||
|226178|missing|FAIL|d:missing_store|
|231018|missing|FAIL|d:missing_store|
|255256|missing|FAIL|d:missing_store|
|255260|Chaos Brand|PASS||
|258920|Immolation Aura|FAIL|a:aura:220|
|278326|Consume Magic|PASS||
|320313|Swallowed Anger|PASS||
|320331|Infernal Armor|PASS||
|320361|Improved Disrupt|PASS||
|320364|Immolation Aura|PASS||
|320386|Bouncing Glaives|PASS||
|320416|Blazing Path|PASS||
|320418|Improved Sigil of Misery|PASS||
|320635|Vengeful Bonds|PASS||
|320654|Pursuit|PASS||
|320770|Unrestrained Fury|PASS||
|344859|Demon's Bite|PASS||
|344862|Chaos Strike|PASS||
|344865|Fel Rush|PASS||
|389695|Will of the Illidari|PASS||
|389696|Illidari Knowledge|PASS||
|389763|Master of the Glaive|PASS||
|389781|Long Night|PASS||
|389783|Pitch Black|PASS||
|389824|Shattered Restoration|PASS||
|389846|Felfire Haste|PASS||
|389849|Lost in Darkness|PASS||
|391397|Erratic Felheart|PASS||
|391409|Aldrachi Design|PASS||
|393822|Internal Struggle|PASS||
|428607|Live by the Glaive|PASS||
|429211|Champion of the Glaive|PASS||
|442290|Art of the Glaive|PASS||
|442492|Incisive Blade|PASS||
|442497|Keen Engagement|PASS||
|442502|Warblade's Hunger|PASS||
|442679|Reaver's Mark|PASS||
|442683|Aldrachi Tactics|PASS||
|442686|Thrill of the Fight|PASS||
|442714|Army Unto Oneself|PASS||
|442718|Fury of the Aldrachi|PASS||
|442736|Incorruptible Spirit|PASS||
|442806|Wounded Quarry|PASS||
|444926|Evasive Action|PASS||
|444931|Unhindered Assault|PASS||
|444997|Preemptive Strike|PASS||
|452402|Demonsurge|PASS||
|452403|Wave of Debilitation|PASS||
|452404|Pursuit of Angriness|PASS||
|452405|Focused Hatred|PASS||
|452406|Set Fire to the Pain|PASS||
|452407|Improved Soul Rending|PASS||
|452408|Burning Blades|PASS||
|452409|Violent Transformation|PASS||
|452410|Enduring Torment|PASS||
|452411|Untethered Fury|PASS||
|452412|Student of Suffering|PASS||
|452413|Flamebound|PASS||
|452414|Monster Rising|PASS||
|452415|Demonic Intensity|PASS||
|462065|Demon Hunter|FAIL|a:aura:320|
|471306|Void Metamorphosis|PASS||
|473662|Consume|PASS||
|473728|Void Ray|PASS||
|1213636|Devourer Demon Hunter|PASS||
|1215546|missing|FAIL|d:missing_store|
|1215617|Predator's Thirst|PASS||
|1217605|Void Metamorphosis|FAIL|d:fixture:InvalidTarget:SPELL_FAILED_CASTER_AURASTATE|
|1221167|Collapsing Star|PASS||
|1223171|Guile|PASS||
|1223918|Waste Not|PASS||
|1226019|Reap|PASS||
|1226033|Eradicate|PASS||
|1227110|missing|FAIL|d:missing_store|
|1227619|Shattered Souls|PASS||
|1227707|Impending Apocalypse|PASS||
|1229376|Single-Button Assistant|PASS||
|1234195|Void Nova|PASS||
|1234796|Shift|PASS||
|1237270|Feast of Souls|PASS||
|1238488|Moment of Craving|PASS||
|1238739|Soulshaper|PASS||
|1238855|Mastery: Monster Within|PASS||
|1239519|Hungering Slash|PASS||
|1239537|Second Helping|PASS||
|1240201|Devourer's Bite|PASS||
|1240202|Voidglare Boon|PASS||
|1240203|Focused Ray|PASS||
|1240204|Star Fragments|PASS||
|1241937|Soul Immolation|PASS||
|1242492|Emptiness|PASS||
|1242495|Calamitous|PASS||
|1242768|Sweet Release|PASS||
|1242956|Voidrage|PASS||
|1244220|Voidpurge|PASS||
|1244222|Devourer's Edge|PASS||
|1244237|Rolling Torment|PASS||
|1244295|Sweet Suffering|PASS||
|1245412|Voidblade|PASS||
|1246167|The Hunt|PASS||
|1246555|Tempered Soul|PASS||
|1246556|Spontaneous Immolation|PASS||
|1246558|Scythe's Embrace|PASS||
|1246562|Duty Eternal|PASS||
|1247534|Soul Glutton|PASS||
|1247538|Demonic Instinct|PASS||
|1251417|Spectral Sight|FAIL|a:aura:100; a:effect:167|
|1253304|Voidfall|PASS||
|1253377|Meteoric Rise|PASS||
|1253391|Meteoric Fall|PASS||
|1253399|Path to Oblivion|PASS||
|1253402|State of Matter|PASS||
|1253415|Celestial Echoes|PASS||
|1253668|Swift Erasure|PASS||
|1253676|Doomsayer|PASS||
|1253769|Catastrophe|PASS||
|1253805|Final Hour|PASS||
|1253817|Otherworldly Focus|PASS||
|1256245|Phase Shift|PASS||
|1256295|Mass Acceleration|PASS||
|1256307|Dark Matter|PASS||
|1256353|World Killer|PASS||
|1256964|Devourer Demon Hunter|FAIL|a:aura:646; a:aura:647; a:aura:648; a:aura:649|
|1256968|Devourer Demon Hunter|PASS||
|1260008|Grim Focus|PASS||
|1261684|Entropy|PASS||
|1266296|Focused Ire|PASS||
|1266307|Demonic Resilience|PASS||
|1266316|Burn It Out|PASS||
|1266326|Furious|PASS||
|1266328|Remorseless|PASS||
|1266329|Demon Muzzle|PASS||
|1266330|Soul Splitter|PASS||
|1266493|Wings of Wrath|PASS||
|1266496|Soul Cleanse|PASS||
|1266497|First In, Last Out|PASS||
|1266500|Final Breath|PASS||
|1272138|Keen Edge|PASS||
|1272143|Broken Spirit|PASS||
|1272153|Bladecraft|PASS||
|1272364|Blind Focus|PASS||
|1272405|Undying Embers|PASS||
|1272422|Voidrush|PASS||
|1272432|Soulforged Blades|PASS||
|1272453|Volatile Instinct|PASS||
|1272770|Singular Strikes|PASS||
|1272817|Improved Consume|PASS||
|1272832|Singed Spirit|PASS||
|1272842|Umbral Blade|PASS||
|1272847|Gift of the Void|PASS||
|1277734|Leather Specialization|FAIL|d:fixture:no_passive_aura|
|1277736|Demonic Wards|PASS||
|1279247|Harness the Cosmos|PASS||
