#[derive(Debug, Clone)]
pub enum Event {
    ShowInteractionPrompt{message: String, key: String},
    Print{message: String},
    DoInteraction{npc: usize},
    ShowDialog{text: String}
}