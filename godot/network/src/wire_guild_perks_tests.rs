//! Owner spell knowledge must retain TalentChannel snapshot/delta ordering.
use super::*;

#[test]
fn guild_perks_wire_preserves_login_quit_and_rejoin_spell_order() {
    use shared::protocol::{KnownSpellsSnapshot, SpellsLearned, SpellsUnlearned, TalentChannel};
    let address: SocketAddr = "127.0.0.1:5891".parse().unwrap();
    let mut server = create_fixture_server(|_| {});
    let entity = server
        .world_mut()
        .spawn((
            network::LocalAddr(address),
            server::ServerUdpIo::default(),
            server::NetcodeServer::new(server::NetcodeConfig::default()),
        ))
        .id();
    server.world_mut().trigger(server::Start { entity });
    let mut host = Host::connect(address, 8261);
    await_connected(&mut server, &mut host);
    let (held, confirmed) = mpsc::channel();
    let (resume, resumed) = mpsc::channel();
    host.bridge
        .enqueue(move |_| {
            held.send(()).unwrap();
            resumed.recv_timeout(Duration::from_secs(10)).unwrap();
        })
        .unwrap();
    confirmed.recv_timeout(Duration::from_secs(10)).unwrap();
    let perks = vec![78633, 83944, 83958, 83950, 83951];
    let login = KnownSpellsSnapshot {
        spells: vec![35395],
    };
    let respec = KnownSpellsSnapshot {
        spells: vec![35395, 19750],
    };
    macro_rules! send_flush {
        ($ty:ty, $message:expr) => {{
            let world = server.world_mut();
            world
                .query::<&mut MessageSender<$ty>>()
                .single_mut(world)
                .unwrap()
                .send::<TalentChannel>($message);
            server.update();
        }};
    }
    // Login snapshot precedes the membership grant; quit revokes it, then a
    // class snapshot and real membership rejoin grant arrive in the same batch.
    send_flush!(KnownSpellsSnapshot, login.clone());
    send_flush!(
        SpellsLearned,
        SpellsLearned {
            spells: perks.clone()
        }
    );
    send_flush!(
        SpellsUnlearned,
        SpellsUnlearned {
            spells: perks.clone()
        }
    );
    send_flush!(KnownSpellsSnapshot, respec.clone());
    send_flush!(
        SpellsLearned,
        SpellsLearned {
            spells: perks.clone()
        }
    );
    let deadline = Instant::now() + Duration::from_millis(100);
    while Instant::now() < deadline {
        server.update();
        thread::sleep(Duration::from_millis(5));
    }
    resume.send(()).unwrap();
    let messages = await_messages(&mut server, &mut host, 5);
    host.stop();
    let mut received = messages.into_iter();
    assert_eq!(
        received
            .next()
            .unwrap()
            .downcast::<KnownSpellsSnapshot>()
            .ok(),
        Some(login)
    );
    assert_eq!(
        received.next().unwrap().downcast::<SpellsLearned>().ok(),
        Some(SpellsLearned {
            spells: perks.clone()
        })
    );
    assert_eq!(
        received.next().unwrap().downcast::<SpellsUnlearned>().ok(),
        Some(SpellsUnlearned {
            spells: perks.clone()
        })
    );
    assert_eq!(
        received
            .next()
            .unwrap()
            .downcast::<KnownSpellsSnapshot>()
            .ok(),
        Some(respec)
    );
    assert_eq!(
        received.next().unwrap().downcast::<SpellsLearned>().ok(),
        Some(SpellsLearned { spells: perks })
    );
}
