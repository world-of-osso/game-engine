//! Retail bag bar art from `UiTextureAtlasMember.csv` on its atlas sheet.

/// Pixel crop (left, right, top, bottom) on a sheet.
#[derive(Clone, Copy)]
pub(super) struct SheetCrop {
    pub fdid: u32,
    pub(super) sheet_w: f32,
    pub(super) sheet_h: f32,
    pub(super) left: f32,
    pub(super) right: f32,
    pub(super) top: f32,
    pub(super) bottom: f32,
}

impl SheetCrop {
    /// Normalized `tex_coords` attribute value.
    pub fn tex_coords(self) -> String {
        format!(
            "{},{},{},{}",
            self.left / self.sheet_w,
            self.right / self.sheet_w,
            self.top / self.sheet_h,
            self.bottom / self.sheet_h
        )
    }
}

/// UiTextureAtlas 2098, 512x128.
const BAG_SHEET: u32 = 4_691_255;

const fn bag(left: f32, right: f32, top: f32, bottom: f32) -> SheetCrop {
    SheetCrop {
        fdid: BAG_SHEET,
        sheet_w: 512.0,
        sheet_h: 128.0,
        left,
        right,
        top,
        bottom,
    }
}

/// `bag-main-2x` (member 16752): the backpack button.
pub(super) const BACKPACK: SheetCrop = bag(1.0, 97.0, 1.0, 97.0);
/// `bag-border-empty-2x` (member 16751): an empty bag slot.
pub(super) const BAG_SLOT_EMPTY: SheetCrop = bag(295.0, 356.0, 64.0, 125.0);
/// `bag-border-2x` (member 16750): a bag slot holding a bag.
pub(super) const BAG_SLOT: SheetCrop = bag(295.0, 356.0, 1.0, 62.0);
/// `bag-reagent-border-2x` (member 16753): the reagent bag slot holding a bag.
pub(super) const REAGENT_SLOT: SheetCrop = bag(358.0, 419.0, 64.0, 125.0);
/// `bag-reagent-border-empty-2x` (member 16754): the empty reagent bag slot.
pub(super) const REAGENT_SLOT_EMPTY: SheetCrop = bag(421.0, 482.0, 1.0, 62.0);
/// `bag-arrow-2x` (member 16749): `BagBarExpandToggle`, pointing left unrotated.
pub(super) const BAG_ARROW: SheetCrop = bag(484.0, 504.0, 1.0, 33.0);
