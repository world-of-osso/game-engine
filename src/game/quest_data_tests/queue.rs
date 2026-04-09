use super::*;

#[test]
fn intent_queue_accept() {
    let mut queue = QuestIntentQueue::default();
    queue.accept(100, 5);
    let drained = queue.drain();
    assert_eq!(drained.len(), 1);
    assert_eq!(
        drained[0],
        QuestIntent::Accept {
            quest_id: 100,
            npc_id: 5
        }
    );
}

#[test]
fn intent_queue_turn_in() {
    let mut queue = QuestIntentQueue::default();
    queue.turn_in(200, 10);
    let drained = queue.drain();
    assert_eq!(
        drained[0],
        QuestIntent::TurnIn {
            quest_id: 200,
            npc_id: 10
        }
    );
}

#[test]
fn intent_queue_abandon() {
    let mut queue = QuestIntentQueue::default();
    queue.abandon(300);
    let drained = queue.drain();
    assert_eq!(drained[0], QuestIntent::Abandon { quest_id: 300 });
    assert!(queue.pending.is_empty());
}
