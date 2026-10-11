//! The agent panel in the full app: where it opens, and that typing in it reaches the agent
//! rather than the tool shortcuts.

use std::sync::{Arc, Mutex};

use egui_kittest::Harness;
use serde_json::json;

use crate::PhotocraftApp;
use crate::agent_panel::{AgentEvent, AgentRequest, AgentRun};
use crate::state::{AgentKind, Tool};

fn harness(with_doc: bool, sent: Arc<Mutex<Vec<String>>>) -> Harness<'static, PhotocraftApp> {
    Harness::builder().with_size(egui::vec2(1400.0, 900.0)).with_max_steps(64).build_eframe(move |cc| {
        PhotocraftApp::setup_context(&cc.egui_ctx, Default::default());
        let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), crate::Services::default());
        if with_doc {
            app.run("file.new", json!({"width": 64, "height": 64})).unwrap();
        }
        app.services.agent = Some(Box::new(move |req: AgentRequest, _ctx| {
            sent.lock().unwrap().push(req.prompt);
            let (tx, rx) = std::sync::mpsc::channel();
            tx.send(AgentEvent::Text("ok".into())).unwrap();
            tx.send(AgentEvent::Done).unwrap();
            Ok(AgentRun { events: rx, cancel: Box::new(|| {}), control: None })
        }));
        app.ui.tool = Tool::Move;
        app.ui.agent_panel = Some(AgentKind::Claude);
        app
    })
}

fn panel_rect(h: &Harness<'_, PhotocraftApp>) -> egui::Rect {
    h.ctx.memory(|m| m.area_rect(egui::Id::new("agent-panel"))).expect("the agent panel is shown")
}

#[test]
fn the_panel_opens_in_the_document_areas_bottom_right_corner() {
    for with_doc in [false, true] {
        let mut h = harness(with_doc, Arc::default());
        h.run_steps(6);
        let area = h.state().agent.area.expect("the document area is recorded");
        let panel = panel_rect(&h);
        assert!(area.contains_rect(panel), "with_doc={with_doc}: panel {panel:?} inside the document area {area:?}");
        assert!((area.right() - panel.right()) < 40.0 && (area.bottom() - panel.bottom()) < 40.0, "with_doc={with_doc}: {panel:?} in {area:?}");
    }
}

#[test]
fn typing_goes_to_the_agent_not_the_tool_shortcuts() {
    let sent = Arc::new(Mutex::new(Vec::new()));
    let mut h = harness(true, Arc::clone(&sent));
    h.run_steps(6);
    h.ctx.memory_mut(|m| m.request_focus(egui::Id::new("agent-input")));
    h.run_steps(2);
    // "b" would pick the Brush tool if the shortcut saw it.
    h.event(egui::Event::Text("blur".into()));
    h.run_steps(2);
    assert_eq!(h.state().ui.tool, Tool::Move);
    h.key_press(egui::Key::Enter);
    h.run_steps(4);
    assert_eq!(*sent.lock().unwrap(), vec!["blur".to_string()]);
    let conv = h.state().agent.conversation(AgentKind::Claude);
    assert_eq!(conv.entries.len(), 2, "the message and the reply: {:?}", conv.entries);
}
