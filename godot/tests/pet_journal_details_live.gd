extends "res://tests/questlog_fidelity_live.gd"

# QF_STEPS=check_skin,check_owned_combat_details. Existing disposable account only.
# Click an unlocked authored choice; flyout closes only on server acknowledgement.
func check_owned_combat_details() -> bool:
	await open_pet_details()
	if not await wait_frames(func(): return client.find_child("PetJournalPetCardHealthValue", true, false) != null, "owned combat snapshot"):
		return false
	var health := client.find_child("PetJournalPetCardHealthValue", true, false) as Label
	var owned_health := health.text
	if not owned_health.is_valid_int() or int(owned_health) <= 0:
		fail("Expected server-owned living pet health")
		return false
	var journal := client.get_node("CollectionsJournalUI")
	await click_control(journal.find_child("PetJournalBattleSlot0", true, false) as Control)
	if not await wait_frames(ability_selector_ready, "server-acknowledged equipped combat details", 60000):
		return false
	print("PET_COMBAT_LOADOUT_ACK parent_disabled=", (journal.find_child("PetJournalBattleSlot0", true, false) as BaseButton).disabled)
	var spell := client.find_child("PetJournalBattleSlot0Spell1Button", true, false) as BaseButton
	if spell == null or spell.disabled:
		fail("Owned ability selector disabled while idle")
		return false
	await click_control(spell)
	if not await wait_frames(func(): return client.find_child("PetJournalSpellSelect", true, false) != null, "Retail spell flyout"):
		return false
	var flyout := client.find_child("PetJournalSpellSelect", true, false)
	var unlocked: BaseButton
	var locked := 0
	for node in flyout.find_children("PetJournalSpellSelectOption*", "BaseButton", true, false):
		var button := node as BaseButton
		if button.disabled:
			locked += 1
		else:
			unlocked = button
	if unlocked == null or locked != 1:
		fail("Expected one unlocked and one level-locked authored choice")
		return false
	await capture("owned-combat-flyout.png")
	var chosen_name := String(unlocked.name)
	print("PET_COMBAT_CLICK_TARGET ", chosen_name, " rect=", unlocked.get_global_rect())
	unlocked.pressed.connect(func(): print("PET_COMBAT_BUTTON_PRESSED ", chosen_name))
	await click_control(unlocked)
	print("PET_COMBAT_HOVER_AFTER_CLICK ", root.gui_get_hovered_control())
	var pending_button := client.find_child(chosen_name, true, false) as BaseButton
	if pending_button != null:
		print("PET_COMBAT_BUTTON_AFTER_CLICK disabled=", pending_button.disabled)
	await capture("owned-combat-after-click.png")
	if not await wait_frames(func(): return client.find_child("PetJournalSpellSelect", true, false) == null, "ability server acknowledgement"):
		return false
	if (client.find_child("PetJournalPetCardHealthValue", true, false) as Label).text != owned_health:
		fail("Ability acknowledgement changed owned health")
		return false
	await capture("owned-combat-ack.png")
	await tap(KEY_ESCAPE)
	await open_pet_details()
	if not await wait_frames(func(): return client.find_child("PetJournalPetCardHealthValue", true, false) != null, "reopened owner details"):
		return false
	if (client.find_child("PetJournalPetCardHealthValue", true, false) as Label).text != owned_health:
		fail("Reopening journal changed owned health")
		return false
	await capture("owned-combat-reopened.png")
	print("PASS PET_COMBAT_DETAILS skin=", OS.get_environment("QF_SKIN"), " health=", owned_health, " locked-choice=1 server-ack=1 reopened=1")
	return true

func ability_selector_ready() -> bool:
	var button := client.find_child("PetJournalBattleSlot0Spell1Button", true, false) as BaseButton
	return button != null and not button.disabled

func open_pet_details() -> void:
	await tap_collections()
	if not await wait_frames(func(): return client.find_child("CollectionsJournalTab2", true, false) != null, "Collections shell"):
		return
	await click_control(client.find_child("CollectionsJournalTab2", true, false) as Control)
	if not await wait_frames(func(): return client.get_node_or_null("CollectionsJournalUI") != null, "Pet Journal"):
		return
	var journal := client.get_node("CollectionsJournalUI")
	await frames(30)
	await capture("owned-combat-open-before-row-wait.png")
	print("PET_COMBAT_CONTROL_NAMES ", journal.find_children("*owned*", "Control", true, false))
	if not await wait_frames(func(): return not journal.find_children("pet_select_owned_*", "Control", true, false).is_empty(), "owned pets", 60000):
		return
	for row in journal.find_children("pet_select_owned_*", "Control", true, false):
		if String(row.name).trim_prefix("pet_select_owned_").is_valid_int():
			await click_control(row as Control)
			return
	fail("Owned pet row missing")
