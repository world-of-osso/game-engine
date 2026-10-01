//! Physical vendor buy -> embedded backpack pickup -> merchant background whole-stack sale.
use super::*;
use shared::protocol::{
    BuyItem, BuybackItem, BuybackItemRequest, BuybackList, DestroyItem, EquipItem,
    InteractionClosed, InventoryDelta, InventorySlotChange, ItemLocation, ItemStack, RepairItem,
    SellAllJunkItems, SellItem, SortBags, SplitItem, SwapItem, UseItem,
};

const QUIET: Duration = Duration::from_millis(400);
const REQUEST_WAIT: Duration = Duration::from_secs(6);
const DESTINATION: ItemLocation = ItemLocation::Bag { bag: 0, slot: 0 };

#[derive(Resource, Default)]
struct Requests {
    buys: Vec<BuyItem>,
    sells: Vec<SellItem>,
    buybacks: Vec<BuybackItemRequest>,
    forbidden: Vec<String>,
}

fn collect_forbidden<M: network::Message + std::fmt::Debug>(
    receivers: &mut Query<&mut MessageReceiver<M>>,
    requests: &mut Requests,
) {
    for mut receiver in receivers.iter_mut() {
        requests.forbidden.extend(
            receiver
                .receive()
                .map(|request| format!("{} {request:?}", std::any::type_name::<M>())),
        );
    }
}

fn receive_merchant(
    mut buys: Query<&mut MessageReceiver<BuyItem>>,
    mut sells: Query<&mut MessageReceiver<SellItem>>,
    mut junk: Query<&mut MessageReceiver<SellAllJunkItems>>,
    mut buybacks: Query<&mut MessageReceiver<BuybackItemRequest>>,
    mut repairs: Query<&mut MessageReceiver<RepairItem>>,
    mut requests: ResMut<Requests>,
) {
    for mut receiver in &mut buys {
        requests.buys.extend(receiver.receive());
    }
    for mut receiver in &mut sells {
        requests.sells.extend(receiver.receive());
    }
    collect_forbidden(&mut junk, &mut requests);
    collect_buybacks(&mut buybacks, &mut requests);
    collect_forbidden(&mut repairs, &mut requests);
}

fn collect_buybacks(
    receivers: &mut Query<&mut MessageReceiver<BuybackItemRequest>>,
    requests: &mut Requests,
) {
    for mut receiver in receivers.iter_mut() {
        requests.buybacks.extend(receiver.receive());
    }
}

fn receive_inventory(
    mut swaps: Query<&mut MessageReceiver<SwapItem>>,
    mut splits: Query<&mut MessageReceiver<SplitItem>>,
    mut destroys: Query<&mut MessageReceiver<DestroyItem>>,
    mut equips: Query<&mut MessageReceiver<EquipItem>>,
    mut uses: Query<&mut MessageReceiver<UseItem>>,
    mut sorts: Query<&mut MessageReceiver<SortBags>>,
    mut requests: ResMut<Requests>,
) {
    collect_forbidden(&mut swaps, &mut requests);
    collect_forbidden(&mut splits, &mut requests);
    collect_forbidden(&mut destroys, &mut requests);
    collect_forbidden(&mut equips, &mut requests);
    collect_forbidden(&mut uses, &mut requests);
    collect_forbidden(&mut sorts, &mut requests);
}

#[derive(Debug, PartialEq, Eq)]
enum Phase {
    Loading,
    Ready,
    Open,
    VendorOpen,
    PickupArm,
    Pickup,
    Held,
    Request,
    Delta,
    BuyDone,
    SalePickupArm,
    SalePickup,
    SaleHeld,
    SaleRequest,
    SaleDelta,
    SplitSeed,
    SplitPickerArm,
    SplitPicker,
    SplitHeld,
    SplitRequest,
    SplitDelta,
    ShiftPickerArm,
    ShiftPicker,
    ShiftCap,
    ShiftRequest,
    ShiftDelta,
    BuybackSeed,
    BuybackTab,
    BuybackRequest,
    BuybackDelta,
    CloseCatalog,
    ClosePickup,
    ClosePressed,
    CloseRelease,
    CloseHeld,
    CloseRequest,
    CloseDelta,
    CloseQuiet,
    Drain,
}

struct Session {
    phase: Phase,
    since: Instant,
    selected: Option<Entity>,
    opens: usize,
    buys: usize,
    sells: usize,
    buybacks: usize,
    closes: usize,
    close_commit: bool,
    buyback_commit: bool,
    commit: bool,
    sell_commit: bool,
    split_sell_commit: bool,
    shift_buy_commit: bool,
}

impl Session {
    fn advance(&mut self, phase: Phase) {
        println!(
            "MERCHANT CURSOR PHASE {phase:?} opens={} buys={} sells={}",
            self.opens, self.buys, self.sells
        );
        self.phase = phase;
        self.since = Instant::now();
    }

    fn require_quiet(&self) -> Result<(), String> {
        if self.since.elapsed() < QUIET {
            return Err(format!(
                "merchant-cursor quiet phase too short: {:?}",
                self.phase
            ));
        }
        Ok(())
    }

    fn send_phase_responses(&mut self, app: &mut App, line: &str) -> Result<(), String> {
        bags::reject_runtime_error(line)?;
        if !line.starts_with("FIXTURE MERCHANT_CURSOR_") {
            return Ok(());
        }
        if self.selected.is_none() {
            return Err(format!(
                "merchant-cursor marker before authenticated selection: {line}"
            ));
        }
        if line.starts_with("FIXTURE MERCHANT_CURSOR_CLOSE_")
            || line == "FIXTURE MERCHANT_CURSOR_DONE"
        {
            self.send_close_marker_response(line)
        } else if line.starts_with("FIXTURE MERCHANT_CURSOR_BUYBACK_") {
            self.send_buyback_marker_response(app, line)
        } else if line.starts_with("FIXTURE MERCHANT_CURSOR_SHIFT_") {
            self.send_shift_buy_marker_response(line)
        } else if line.starts_with("FIXTURE MERCHANT_CURSOR_SPLIT_") {
            self.send_split_marker_response(app, line)
        } else if line.starts_with("FIXTURE MERCHANT_CURSOR_SALE_")
            || line == "FIXTURE MERCHANT_CURSOR_SELL_COMMIT"
        {
            self.send_whole_sale_marker_response(line)
        } else {
            self.send_buy_marker_response(app, line)
        }
    }

    fn send_buy_marker_response(&mut self, app: &mut App, line: &str) -> Result<(), String> {
        match (&self.phase, line) {
            (Phase::Loading, "FIXTURE MERCHANT_CURSOR_LOADING") => {
                let selected = self.selected.ok_or("Loading requires selected player")?;
                app.world_mut().entity_mut(selected).insert(Gold(1000));
                println!(
                    "MERCHANT CURSOR INITIAL selected={} Gold1000",
                    selected.to_bits()
                );
                send::<_, TerrainChannel>(
                    app,
                    LoadTerrain {
                        map_name: "azeroth".into(),
                        initial_tile_y: 32,
                        initial_tile_x: 48,
                    },
                );
                self.advance(Phase::Ready);
            }
            (Phase::Ready, "FIXTURE MERCHANT_CURSOR_READY") => self.advance(Phase::Open),
            (Phase::VendorOpen, "FIXTURE MERCHANT_CURSOR_VENDOR_OPEN") => {
                self.advance(Phase::PickupArm)
            }
            (Phase::PickupArm, "FIXTURE MERCHANT_CURSOR_PICKUP_ARM") => self.advance(Phase::Pickup),
            (Phase::Pickup, "FIXTURE MERCHANT_CURSOR_PICKED_UP") => {
                self.require_quiet()?;
                self.advance(Phase::Held);
            }
            (Phase::Held, "FIXTURE MERCHANT_CURSOR_DROP_ARM") => self.advance(Phase::Request),
            (Phase::Request, "FIXTURE MERCHANT_CURSOR_COMMIT") if !self.commit => {
                self.require_quiet()?;
                self.commit = true;
            }
            (Phase::Delta, "FIXTURE MERCHANT_CURSOR_BUY_DONE") => {
                self.require_quiet()?;
                self.advance(Phase::BuyDone);
            }
            _ => return Err(self.marker_order_error(line)),
        }
        Ok(())
    }

    fn send_whole_sale_marker_response(&mut self, line: &str) -> Result<(), String> {
        match (&self.phase, line) {
            (Phase::BuyDone, "FIXTURE MERCHANT_CURSOR_SALE_PICKUP_ARM") => {
                self.advance(Phase::SalePickupArm)
            }
            (Phase::SalePickupArm, "FIXTURE MERCHANT_CURSOR_SALE_PRESS_ARM") => {
                self.advance(Phase::SalePickup)
            }
            (Phase::SalePickup, "FIXTURE MERCHANT_CURSOR_SALE_PICKED_UP") => {
                self.require_quiet()?;
                self.advance(Phase::SaleHeld);
            }
            (Phase::SaleHeld, "FIXTURE MERCHANT_CURSOR_SALE_DROP_ARM") => {
                self.advance(Phase::SaleRequest)
            }
            (Phase::SaleRequest, "FIXTURE MERCHANT_CURSOR_SELL_COMMIT") if !self.sell_commit => {
                self.require_quiet()?;
                self.sell_commit = true;
            }
            _ => return Err(self.marker_order_error(line)),
        }
        Ok(())
    }

    fn send_split_marker_response(&mut self, app: &mut App, line: &str) -> Result<(), String> {
        match (&self.phase, line) {
            (Phase::SaleDelta, "FIXTURE MERCHANT_CURSOR_SPLIT_SEED") => {
                self.require_quiet()?;
                if !self.completed_whole_sale() {
                    return Err("split seed requires completed buy and whole-sale barriers".into());
                }
                send_split_stack(app, 5);
                self.advance(Phase::SplitSeed);
            }
            (Phase::SplitSeed, "FIXTURE MERCHANT_CURSOR_SPLIT_PICKER_ARM") => {
                self.require_quiet()?;
                self.advance(Phase::SplitPickerArm);
            }
            (Phase::SplitPickerArm, "FIXTURE MERCHANT_CURSOR_SPLIT_PICKER_OPEN") => {
                self.require_quiet()?;
                self.advance(Phase::SplitPicker);
            }
            (Phase::SplitPicker, "FIXTURE MERCHANT_CURSOR_SPLIT_HELD") => {
                self.require_quiet()?;
                self.advance(Phase::SplitHeld);
            }
            (Phase::SplitHeld, "FIXTURE MERCHANT_CURSOR_SPLIT_SELL_PRESS_ARM") => {
                self.require_quiet()?;
                self.advance(Phase::SplitRequest);
            }
            (Phase::SplitRequest, "FIXTURE MERCHANT_CURSOR_SPLIT_SELL_COMMIT")
                if !self.split_sell_commit =>
            {
                self.require_quiet()?;
                self.split_sell_commit = true;
            }
            _ => return Err(self.marker_order_error(line)),
        }
        Ok(())
    }

    fn send_shift_buy_marker_response(&mut self, line: &str) -> Result<(), String> {
        match (&self.phase, line) {
            (Phase::SplitDelta, "FIXTURE MERCHANT_CURSOR_SHIFT_BUY_ARM") => {
                self.require_quiet()?;
                if !self.completed_split_sale() {
                    return Err("Shift buy requires completed original three barriers".into());
                }
                self.advance(Phase::ShiftPickerArm);
            }
            (Phase::ShiftPickerArm, "FIXTURE MERCHANT_CURSOR_SHIFT_PICKER_OPEN") => {
                self.require_quiet()?;
                self.advance(Phase::ShiftPicker);
            }
            (Phase::ShiftPicker, "FIXTURE MERCHANT_CURSOR_SHIFT_CAP40") => {
                self.require_quiet()?;
                self.advance(Phase::ShiftCap);
            }
            (Phase::ShiftCap, "FIXTURE MERCHANT_CURSOR_SHIFT_REQUEST_ARM") => {
                self.advance(Phase::ShiftRequest);
            }
            (Phase::ShiftRequest, "FIXTURE MERCHANT_CURSOR_SHIFT_BUY_COMMIT")
                if !self.shift_buy_commit =>
            {
                self.require_quiet()?;
                self.shift_buy_commit = true;
            }
            _ => return Err(self.marker_order_error(line)),
        }
        Ok(())
    }

    fn send_buyback_marker_response(&mut self, app: &mut App, line: &str) -> Result<(), String> {
        match (&self.phase, line) {
            (Phase::ShiftDelta, "FIXTURE MERCHANT_CURSOR_BUYBACK_SEED") => {
                self.require_quiet()?;
                if !self.completed_shift_buy() {
                    return Err("buyback seed requires all four original barriers".into());
                }
                let vendor = app
                    .world()
                    .resource::<Incoming>()
                    .vendor
                    .map(Entity::to_bits);
                if vendor != Some(4_294_966_979) {
                    return Err(format!(
                        "buyback seed requires exact owned vendor: {vendor:?}"
                    ));
                }
                send_buyback_seed(app);
                self.advance(Phase::BuybackSeed);
            }
            (Phase::BuybackSeed, "FIXTURE MERCHANT_CURSOR_BUYBACK_TAB_OPEN") => {
                self.require_quiet()?;
                self.advance(Phase::BuybackTab);
            }
            (Phase::BuybackTab, "FIXTURE MERCHANT_CURSOR_BUYBACK_REQUEST_ARM") => {
                self.advance(Phase::BuybackRequest);
            }
            (Phase::BuybackRequest, "FIXTURE MERCHANT_CURSOR_BUYBACK_COMMIT")
                if !self.buyback_commit =>
            {
                self.require_quiet()?;
                self.buyback_commit = true;
            }
            (Phase::BuybackDelta, "FIXTURE MERCHANT_CURSOR_DONE") => {
                self.require_quiet()?;
                self.advance(Phase::Drain);
            }
            _ => return Err(self.marker_order_error(line)),
        }
        Ok(())
    }

    fn send_close_marker_response(&mut self, line: &str) -> Result<(), String> {
        match (&self.phase, line) {
            (Phase::BuybackDelta, "FIXTURE MERCHANT_CURSOR_CLOSE_CATALOG_ARM") => {
                self.require_quiet()?;
                if !self.completed_five_cases() {
                    return Err("vendor close requires all five inventory barriers".into());
                }
                self.advance(Phase::CloseCatalog);
            }
            (Phase::CloseCatalog, "FIXTURE MERCHANT_CURSOR_CLOSE_PICKUP_ARM") => {
                self.require_quiet()?;
                self.advance(Phase::ClosePickup);
            }
            (Phase::ClosePickup, "FIXTURE MERCHANT_CURSOR_CLOSE_PICKED_UP") => {
                self.require_quiet()?;
                self.advance(Phase::ClosePressed);
            }
            (Phase::ClosePressed, "FIXTURE MERCHANT_CURSOR_CLOSE_RELEASE_ARM") => {
                self.advance(Phase::CloseRelease);
            }
            (Phase::CloseRelease, "FIXTURE MERCHANT_CURSOR_CLOSE_HELD") => {
                self.require_quiet()?;
                self.advance(Phase::CloseHeld);
            }
            (Phase::CloseHeld, "FIXTURE MERCHANT_CURSOR_CLOSE_REQUEST_ARM") => {
                self.advance(Phase::CloseRequest);
            }
            (Phase::CloseRequest, "FIXTURE MERCHANT_CURSOR_CLOSE_COMMIT") if !self.close_commit => {
                self.require_quiet()?;
                self.close_commit = true;
            }
            (Phase::CloseDelta, "FIXTURE MERCHANT_CURSOR_CLOSE_ACK_QUIET_ARM") => {
                self.require_quiet()?;
                self.advance(Phase::CloseQuiet);
            }
            (Phase::CloseQuiet, "FIXTURE MERCHANT_CURSOR_DONE") => {
                self.require_quiet()?;
                self.advance(Phase::Drain);
            }
            _ => return Err(self.marker_order_error(line)),
        }
        Ok(())
    }

    fn completed_five_cases(&self) -> bool {
        self.completed_shift_buy() && self.buybacks == 1 && self.buyback_commit
    }

    fn completed_close(&self) -> bool {
        self.completed_five_cases() && self.closes == 1 && self.close_commit
    }

    fn completed_shift_buy(&self) -> bool {
        self.opens == 1
            && self.buys == 2
            && self.sells == 2
            && self.commit
            && self.sell_commit
            && self.split_sell_commit
            && self.shift_buy_commit
    }

    fn completed_split_sale(&self) -> bool {
        self.completed_purchase() && self.sells == 2 && self.sell_commit && self.split_sell_commit
    }

    fn marker_order_error(&self, line: &str) -> String {
        format!(
            "out-of-order merchant-cursor marker in {:?}: {line}",
            self.phase
        )
    }

    fn completed_purchase(&self) -> bool {
        self.buys == 1 && self.commit
    }

    fn completed_whole_sale(&self) -> bool {
        self.completed_purchase() && self.sells == 1 && self.sell_commit
    }

    fn whole_sale_phase_ready(&self) -> bool {
        self.phase == Phase::SaleRequest && self.sells == 0 && self.completed_purchase()
    }

    fn respond(&mut self, app: &mut App) -> Result<(), String> {
        self.respond_to_interaction(app)?;
        self.receive_requests(app)?;
        if self.phase == Phase::Request {
            if self.commit && self.buys == 1 {
                send_purchase(
                    app,
                    self.selected.ok_or("purchase requires selected player")?,
                );
                self.advance(Phase::Delta);
            } else if self.since.elapsed() > REQUEST_WAIT {
                return Err(format!(
                    "merchant-cursor missing exact BuyItem/COMMIT; buys={} commit={}",
                    self.buys, self.commit
                ));
            }
        }
        if self.phase == Phase::SaleRequest {
            if self.sell_commit && self.sells == 1 {
                send_sale(app, self.selected.ok_or("sale requires selected player")?);
                self.advance(Phase::SaleDelta);
            } else if self.since.elapsed() > REQUEST_WAIT {
                return Err(format!(
                    "merchant-cursor missing exact SellItem/SELL_COMMIT; sells={} commit={}",
                    self.sells, self.sell_commit
                ));
            }
        }
        if self.phase == Phase::SplitRequest {
            if self.split_sell_commit && self.sells == 2 {
                send_split_stack(app, 3);
                app.world_mut()
                    .entity_mut(self.selected.ok_or("split sale requires selected player")?)
                    .insert(Gold(1014));
                println!(
                    "MERCHANT CURSOR SPLIT AUTHORITATIVE Linen3 guid9182590 Gold1014; 988+13*2"
                );
                self.advance(Phase::SplitDelta);
            } else if self.since.elapsed() > REQUEST_WAIT {
                return Err(format!(
                    "merchant-cursor missing exact split SellItem/SPLIT_SELL_COMMIT; sells={} commit={}",
                    self.sells, self.split_sell_commit
                ));
            }
        }
        self.respond_to_shift_buy(app)?;
        self.respond_to_buyback(app)?;
        self.respond_to_close(app)
    }

    fn respond_to_shift_buy(&mut self, app: &mut App) -> Result<(), String> {
        if self.phase != Phase::ShiftRequest {
            return Ok(());
        }
        if self.shift_buy_commit && self.buys == 2 {
            // Fixture authority only: ordinary delta into the same GUID-backed stack.
            send_split_stack(app, 5);
            app.world_mut()
                .entity_mut(self.selected.ok_or("Shift buy requires selected player")?)
                .insert(Gold(964));
            println!(
                "MERCHANT CURSOR SHIFT AUTHORITATIVE Linen5 guid9182590 Gold964; fixture stack/pricing, NOT production server proof"
            );
            self.advance(Phase::ShiftDelta);
        } else if self.since.elapsed() > REQUEST_WAIT {
            return Err(format!(
                "merchant-cursor missing exact Shift BuyItem/SHIFT_BUY_COMMIT; buys={} commit={}",
                self.buys, self.shift_buy_commit
            ));
        }
        Ok(())
    }

    fn respond_to_buyback(&mut self, app: &mut App) -> Result<(), String> {
        if self.phase != Phase::BuybackRequest {
            return Ok(());
        }
        if self.buyback_commit && self.buybacks == 1 {
            send_buyback_commit(
                app,
                self.selected.ok_or("buyback requires selected player")?,
            );
            self.advance(Phase::BuybackDelta);
        } else if self.since.elapsed() > REQUEST_WAIT {
            return Err(format!(
                "merchant-cursor missing exact BuybackItemRequest/BUYBACK_COMMIT; buybacks={} commit={}",
                self.buybacks, self.buyback_commit
            ));
        }
        Ok(())
    }

    fn respond_to_close(&mut self, app: &mut App) -> Result<(), String> {
        if self.phase != Phase::CloseRequest {
            return Ok(());
        }
        if self.completed_close() {
            send_close_ack(app);
            self.advance(Phase::CloseDelta);
        } else if self.since.elapsed() > REQUEST_WAIT {
            return Err(format!(
                "merchant-cursor missing exact CloseInteraction/CLOSE_COMMIT; closes={} commit={}",
                self.closes, self.close_commit
            ));
        }
        Ok(())
    }

    fn receive_close(
        &mut self,
        request: CloseInteraction,
        vendor: Option<u64>,
    ) -> Result<(), String> {
        let expected = CloseInteraction { npc: 4_294_966_979 };
        if self.phase != Phase::CloseRequest
            || self.closes != 0
            || !self.completed_five_cases()
            || vendor != Some(expected.npc)
            || request.npc != expected.npc
        {
            return Err(format!(
                "forbidden/wrong-NPC/duplicate CloseInteraction {request:?} in {:?}; closes={}",
                self.phase, self.closes
            ));
        }
        self.closes += 1;
        println!(
            "MERCHANT CURSOR CLOSE DECODED {request:?} count=1; InteractionClosed withheld until pre-ack CLOSE_COMMIT"
        );
        Ok(())
    }

    fn respond_to_interaction(&mut self, app: &mut App) -> Result<(), String> {
        let (vendor, interactions, closes, casts) = {
            let mut incoming = app.world_mut().resource_mut::<Incoming>();
            (
                incoming.vendor,
                std::mem::take(&mut incoming.interactions),
                std::mem::take(&mut incoming.closes),
                std::mem::take(&mut incoming.casts),
            )
        };
        if !casts.is_empty() {
            return Err(format!(
                "merchant-cursor unrelated close/spell requests: {closes:?} {casts:?}"
            ));
        }
        for request in closes {
            self.receive_close(request, vendor.map(Entity::to_bits))?;
        }
        for request in interactions {
            if self.phase != Phase::Open
                || self.opens != 0
                || Some(request.npc) != vendor.map(Entity::to_bits)
            {
                return Err(format!(
                    "unexpected merchant-cursor interaction {request:?} in {:?}",
                    self.phase
                ));
            }
            self.opens += 1;
            send_vendor(app, request.npc);
            self.advance(Phase::VendorOpen);
        }
        Ok(())
    }

    fn receive_requests(&mut self, app: &mut App) -> Result<(), String> {
        let requests = std::mem::take(&mut *app.world_mut().resource_mut::<Requests>());
        if !requests.forbidden.is_empty() {
            return Err(format!(
                "forbidden merchant-cursor requests: {:?}",
                requests.forbidden
            ));
        }
        let vendor = app
            .world()
            .resource::<Incoming>()
            .vendor
            .map(Entity::to_bits);
        for request in requests.buybacks {
            self.receive_buyback(request, vendor)?;
        }
        for request in requests.buys {
            if self.phase == Phase::ShiftRequest {
                self.receive_shift_buy(request, vendor)?;
                continue;
            }
            let expected = BuyItem {
                npc: vendor.ok_or("BuyItem requires owned vendor")?,
                slot: 0,
                item_id: 2589,
                count: 1,
                destination: Some(DESTINATION),
            };
            if self.phase != Phase::Request || self.buys != 0 || request != expected {
                return Err(format!(
                    "unexpected/duplicate BuyItem {request:?} in {:?}; count={}",
                    self.phase, self.buys
                ));
            }
            self.buys += 1;
            println!(
                "MERCHANT CURSOR DECODED {request:?} count=1; delta and Gold withheld until COMMIT"
            );
        }
        for request in requests.sells {
            if self.phase == Phase::SplitRequest {
                self.receive_split_sale(request, vendor)?;
                continue;
            }
            let npc = vendor.ok_or("SellItem requires owned vendor")?;
            if !self.whole_sale_phase_ready() || !whole_sale_fields_match(&request, npc) {
                return Err(format!(
                    "unexpected/duplicate SellItem {request:?} in {:?}; count={}",
                    self.phase, self.sells
                ));
            }
            self.sells += 1;
            println!(
                "MERCHANT CURSOR DECODED {request:?} count=1; delta and Gold withheld until SELL_COMMIT"
            );
        }
        Ok(())
    }

    fn receive_buyback(
        &mut self,
        request: BuybackItemRequest,
        vendor: Option<u64>,
    ) -> Result<(), String> {
        let expected = BuybackItemRequest {
            npc: 4_294_966_979,
            slot: 0,
        };
        if self.phase != Phase::BuybackRequest
            || self.buybacks != 0
            || !self.completed_shift_buy()
            || vendor != Some(expected.npc)
            || request != expected
        {
            return Err(format!(
                "unexpected/duplicate BuybackItemRequest {request:?} in {:?}; buybacks={}",
                self.phase, self.buybacks
            ));
        }
        self.buybacks += 1;
        println!(
            "MERCHANT CURSOR BUYBACK DECODED {request:?}; Linen5/Gold964/list count2 withheld until BUYBACK_COMMIT"
        );
        Ok(())
    }

    fn receive_shift_buy(&mut self, request: BuyItem, vendor: Option<u64>) -> Result<(), String> {
        let expected = BuyItem {
            npc: vendor.ok_or("Shift BuyItem requires owned vendor")?,
            slot: 0,
            item_id: 2589,
            count: 2,
            destination: None,
        };
        if !self.completed_split_sale() || request != expected {
            return Err(format!(
                "unexpected/duplicate Shift BuyItem {request:?} in {:?}; buys={}",
                self.phase, self.buys
            ));
        }
        self.buys += 1;
        println!(
            "MERCHANT CURSOR SHIFT DECODED {request:?}; Linen3/Gold1014 withheld until SHIFT_BUY_COMMIT"
        );
        Ok(())
    }

    fn receive_split_sale(&mut self, request: SellItem, vendor: Option<u64>) -> Result<(), String> {
        let expected = SellItem {
            npc: vendor.ok_or("split SellItem requires owned vendor")?,
            item_guid: 9_182_590,
            count: 2,
        };
        if !self.completed_whole_sale() || request != expected {
            return Err(format!(
                "unexpected/duplicate split SellItem {request:?} in {:?}; sells={}",
                self.phase, self.sells
            ));
        }
        self.sells += 1;
        println!(
            "MERCHANT CURSOR SPLIT DECODED {request:?}; Linen5/Gold988 withheld until SPLIT_SELL_COMMIT"
        );
        Ok(())
    }
}

fn whole_sale_fields_match(request: &SellItem, npc: u64) -> bool {
    *request
        == SellItem {
            npc,
            item_guid: 9_182_589,
            count: 0,
        }
}

fn send_split_stack(app: &mut App, count: u32) {
    send::<_, InventoryChannel>(
        app,
        InventoryDelta {
            changes: vec![InventorySlotChange {
                location: DESTINATION,
                item: Some(ItemStack {
                    item_guid: 9_182_590,
                    item_id: 2589,
                    count,
                    durability: None,
                    soulbound: false,
                }),
            }],
        },
    );
    println!("MERCHANT CURSOR SPLIT InventoryDelta bag0/slot0 Linen2589 guid9182590 count{count}");
}

fn send_close_ack(app: &mut App) {
    send::<_, InteractionChannel>(app, InteractionClosed { npc: 4_294_966_979 });
    println!(
        "MERCHANT CURSOR CLOSE ACK InteractionClosed npc4294966979; only after exact request and pre-ack CLOSE_COMMIT, no inventory/Gold input"
    );
}

fn send_buyback_seed(app: &mut App) {
    send::<_, MerchantChannel>(
        app,
        BuybackList {
            items: vec![BuybackItem {
                slot: 0,
                item_id: 2589,
                name: "Linen Cloth".into(),
                quality: 1,
                count: 2,
                price: 26,
            }],
        },
    );
    println!(
        "MERCHANT CURSOR BUYBACK SEED slot0 Linen2589 quality1 count2 price26; inventory/Gold unchanged"
    );
}

fn send_buyback_commit(app: &mut App, selected: Entity) {
    // Explicit peer inputs, not production pricing or automatic stacking proof.
    send_split_stack(app, 7);
    app.world_mut().entity_mut(selected).insert(Gold(938));
    send::<_, MerchantChannel>(app, BuybackList::default());
    println!(
        "MERCHANT CURSOR BUYBACK AUTHORITATIVE Linen7 guid9182590 Gold938 empty BuybackList; fixture-only price26/count7"
    );
}

fn send_vendor(app: &mut App, npc: u64) {
    send::<_, InteractionChannel>(
        app,
        InteractionOpened {
            npc,
            kind: InteractionKind::Role(NpcRole::Vendor),
        },
    );
    send::<_, InventoryChannel>(
        app,
        InventorySnapshot {
            bags: vec![BagContents {
                bag: 0,
                size: 16,
                items: vec![],
            }],
        },
    );
    // Local ItemSparse row2589: Stackable1000, VendorStackCount1, quality1.
    send::<_, MerchantChannel>(
        app,
        VendorInventory {
            npc,
            can_repair: false,
            items: vec![VendorItem {
                slot: 0,
                item_id: 2589,
                name: "Linen Cloth".into(),
                quality: 1,
                price: 25,
                stack_count: 1,
                max_stack: 1000,
                num_available: None,
                usable: true,
            }],
        },
    );
}

fn send_purchase(app: &mut App, selected: Entity) {
    send::<_, InventoryChannel>(
        app,
        InventoryDelta {
            changes: vec![InventorySlotChange {
                location: DESTINATION,
                item: Some(ItemStack {
                    item_guid: 9_182_589,
                    item_id: 2589,
                    count: 1,
                    durability: None,
                    soulbound: false,
                }),
            }],
        },
    );
    app.world_mut().entity_mut(selected).insert(Gold(975));
    println!("MERCHANT CURSOR AUTHORITATIVE bag0/slot0 guid9182589 Linen2589 count1 Gold975");
}

fn send_sale(app: &mut App, selected: Entity) {
    send::<_, InventoryChannel>(
        app,
        InventoryDelta {
            changes: vec![InventorySlotChange {
                location: DESTINATION,
                item: None,
            }],
        },
    );
    // Local ItemSparse row2589 SellPrice13, one authoritative Linen Cloth.
    app.world_mut().entity_mut(selected).insert(Gold(988));
    println!("MERCHANT CURSOR AUTHORITATIVE bag0/slot0 empty Gold988; Linen1 SellPrice13");
}

fn tick_peer(
    app: &mut App,
    session: &mut Session,
    remote: &mut Option<Entity>,
) -> Result<(), String> {
    app.update();
    respond_to_login(app, StartupScreen::MerchantCursor)?;
    respond_to_selection(
        app,
        StartupScreen::MerchantCursor,
        &mut session.selected,
        remote,
    )?;
    Ok(())
}

fn run_until_done(
    app: &mut App,
    child: &mut Child,
    lines: &Receiver<String>,
) -> Result<(), String> {
    app.init_resource::<Requests>();
    app.add_systems(Update, (receive_merchant, receive_inventory));
    let mut remote = None;
    let mut session = Session {
        phase: Phase::Loading,
        since: Instant::now(),
        selected: None,
        opens: 0,
        buys: 0,
        sells: 0,
        buybacks: 0,
        closes: 0,
        close_commit: false,
        buyback_commit: false,
        commit: false,
        sell_commit: false,
        split_sell_commit: false,
        shift_buy_commit: false,
    };
    let deadline = Instant::now() + TIMEOUT + Duration::from_secs(180);
    while Instant::now() < deadline {
        tick_peer(app, &mut session, &mut remote)?;
        for line in lines.try_iter() {
            session.send_phase_responses(app, line.trim())?;
        }
        session.respond(app)?;
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("inspect merchant-cursor child: {error}"))?
        {
            return Err(format!(
                "merchant-cursor exited before deliberate cleanup: {status}; phase={:?}",
                session.phase
            ));
        }
        if session.phase == Phase::Drain && session.since.elapsed() >= QUIET {
            if !session.completed_close() {
                return Err(
                    "merchant-cursor final interaction/request/barrier totals failed".into(),
                );
            }
            return Ok(());
        }
        thread::sleep(TICK);
    }
    Err(format!(
        "merchant-cursor timed out; phase={:?} opens={} buys={} sells={}",
        session.phase, session.opens, session.buys, session.sells
    ))
}

pub(super) fn run(
    app: &mut App,
    child: &mut Child,
    lines: Receiver<String>,
    readers: Vec<thread::JoinHandle<()>>,
) -> Result<(), String> {
    let mut result = run_until_done(app, child, &lines);
    let cleanup = bags::cleanup_owned_child(child, readers);
    for line in lines.try_iter() {
        if let Err(error) = bags::reject_runtime_error(line.trim()) {
            result = Err(match result {
                Ok(()) => error,
                Err(original) => format!("{original}; {error}"),
            });
        } else if line.trim().starts_with("FIXTURE MERCHANT_CURSOR_") {
            result = Err(format!(
                "unexpected merchant-cursor marker after cleanup: {}",
                line.trim()
            ));
        }
    }
    match (result, cleanup) {
        (Err(error), Err(cleanup)) => Err(format!("{error}; owned cleanup failed: {cleanup}")),
        (Err(error), _) | (_, Err(error)) => Err(error),
        (Ok(()), Ok(())) => {
            println!(
                "PASS: MERCHANT_CURSOR physical vendor pickup, exact once BuyItem destination bag0/slot0, pre-delta COMMIT empty/1000/cursor cleared, authoritative Linen1/975; physical locked bag0/slot0 pickup to own merchant background, exact once SellItem guid9182589 count0, pre-delta SELL_COMMIT Linen1/975/cursor cleared, authoritative empty/988; seeded Linen5/988, physical Shift-left authored owner-slot picker and digit2/Enter, already-held cursor background press, exact once SellItem guid9182590 count2, pre-delta SPLIT_SELL_COMMIT Linen5/988/unlocked/cursor hidden, authoritative Linen3/1014; physical Shift-vendor buy, MerchantUI-owned BOTTOMLEFT=MerchantItem1 TOPLEFT picker172x96, digits40/Up clamp40/Backspace4/1/digit2/Enter, exact once BuyItem count2 destinationNone, pre-delta SHIFT_BUY_COMMIT Linen3/1014/source white/no cursor, fixture authoritative same guid9182590 Linen5/964; seeded BuybackList slot0 Linen2589 quality1 count2 price26, physical own Buyback tab then cell Left press/release, exact once BuybackItemRequest npc4294966979 slot0, pre-delta BUYBACK_COMMIT Linen5/964/list count2/no held/no picker, fixture authoritative same guid9182590 Linen7/938/empty list/tab retained, physical Tab1 restores vendor catalog, Left hold900ms textured centered Linen vendor cursor, same-source same-point release <4logical px retains cursor quiet900ms/no extra Buy; own CloseButton Left press/release, exact once CloseInteraction npc4294966979 only CloseRequest, pre-ack CLOSE_COMMIT locally closed/merchant and backpack controls hidden (MerchantUI root may remain)/cursor hidden/no picker/popup/Menu/unchanged Linen7 Gold938 quiet900ms, ordinary InteractionClosed only after request plus marker then closed quiet900ms DONE; opens1/buys2/sells2/buybacks1/closes1/five inventory barriers plus close barrier and400ms drain; deliberate kill/reap/readers drained, NOT normal shutdown or full cursor acceptance"
            );
            Ok(())
        }
    }
}
