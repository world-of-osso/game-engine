//! Authored Retail70448 TalkingHeadFrame. Forever changes art, not geometry.
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

pub fn talking_head_screen(_context: &SharedContext) -> Element {
    rsx! {r#frame {name:"TalkingHeadFrame",width:0.0,height:0.0}}
}
