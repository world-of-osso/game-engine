//! Authored Retail70448 TalkingHeadFrame. Forever changes art, not geometry.
use crate::talking_head::TalkingHeadState;
use crate::ui::{strata::FrameStrata, widgets::font_string::GameFont};
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

pub const DISMISS: &str = "talking-head:dismiss";

pub fn talking_head_screen(context: &SharedContext) -> Element {
    let state = context
        .get::<TalkingHeadState>()
        .expect("TalkingHeadState in SharedContext");
    let view = state.current();
    let hidden = view.is_none();
    let name = view.as_ref().map_or("", |view| view.name.as_str());
    let text = view.as_ref().map_or("", |view| view.text.as_str());
    let elapsed = view.as_ref().map_or(0, |view| view.conversation_elapsed_ms);
    // Retail70448 FadeinFrames delays name/text; XML owns each duration.
    let name_alpha = opening_alpha(elapsed, 500, 250);
    let text_alpha = opening_alpha(elapsed, 750, 250);
    let background_alpha = opening_alpha(elapsed, 400, 750);
    let portrait_alpha = opening_alpha(elapsed, 0, 750);
    let close_alpha = opening_alpha(elapsed, 750, 750);
    // TalkingHeadUI.xml: portrait143x143 TOPLEFT5/-6. Lua Name TOPLEFT
    // relative to Portrait TOPRIGHT +2/-19; right edge42; text gap3.
    rsx! {r#frame {name:"TalkingHeadFrame",width:570.0,height:155.0,
        hidden,strata:FrameStrata::High,pos_type:"absolute",left:"50%",translate_x:"-50%",bottom:96.0,
        texture {name:"TalkingHeadTextBackground",texture_atlas:"TalkingHeads-TextBackground",alpha:background_alpha,width:570.0,height:155.0,pos_type:"absolute",left:0.0,top:0.0,draw_layer:"BACKGROUND",mouse_enabled:false}
        r#frame {name:"TalkingHeadModel",alpha:portrait_alpha,width:115.0,height:115.0,left:21.0,top:21.0,pos_type:"absolute",mouse_enabled:false,
            texture {name:"TalkingHeadPortraitBg",texture_atlas:"TalkingHeads-PortraitBg",alpha:portrait_alpha,width:115.0,height:115.0,left:0.0,top:0.0,pos_type:"absolute",draw_layer:"BACKGROUND",mouse_enabled:false}
        }
        texture {name:"TalkingHeadPortrait",texture_atlas:"TalkingHeads-Alliance-PortraitFrame",alpha:portrait_alpha,width:143.0,height:143.0,left:5.0,top:6.0,pos_type:"absolute",draw_layer:"OVERLAY",mouse_enabled:false}
        fontstring {name:"TalkingHeadName",text:name,alpha:name_alpha,font:GameFont::Morpheus,font_size:22.0,font_color:"1,0.82,0,1",justify_h:"LEFT",left:150.0,right:42.0,top:25.0,height:22.0,pos_type:"absolute",mouse_enabled:false}
        fontstring {name:"TalkingHeadText",text,alpha:text_alpha,width:378.0,word_wrap:true,font:GameFont::FrizQuadrata,font_size:16.0,font_color:"1,1,1,1",justify_h:"LEFT",justify_v:"TOP",left:150.0,right:42.0,top:50.0,bottom:12.0,pos_type:"absolute",mouse_enabled:false}
        button {name:"TalkingHeadCloseButton",alpha:close_alpha,width:24.0,height:24.0,right:12.0,top:12.0,pos_type:"absolute",onclick:DISMISS,button_default_skin:false,
            button_atlas_up:"RedButton-Exit",button_atlas_pressed:"RedButton-exit-pressed",button_atlas_disabled:"RedButton-Exit-Disabled",button_atlas_highlight:"RedButton-Highlight",button_highlight_blend_mode:"ADD",button_highlight_alpha:"1.0"}
    }}
}

fn opening_alpha(elapsed_ms: u32, delay_ms: u32, duration_ms: u32) -> f32 {
    if elapsed_ms < delay_ms {
        // TalkingHeadFrameMixin:Reset leaves delayed regions at source0.01.
        return 0.01;
    }
    ((elapsed_ms - delay_ms) as f32 / duration_ms as f32).min(1.0)
}
