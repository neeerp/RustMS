//! Area bosses: each keeps a spot on its map, which the world's maps don't
//! place, and comes back to it as any boss does. The harness world fills
//! every spawn point at once (`RUSTMS_FULL_SPAWN`), so they are out as their
//! maps load.

use integration_harness::events::waiting_in;
use integration_harness::trace::{until, warp, Trace};
use net::mobs::area_boss_on;

#[tokio::test]
async fn area_bosses_are_out_on_their_maps() {
    let mut a = waiting_in(100000000, 30, &[]).await;
    // Mano, and Stumpy, whose spot is the middle of the line Cosmic picks
    // from.
    let mano = area_boss_on(104000400, 2220000).unwrap();
    let stumpy = area_boss_on(101030404, 3220000).unwrap();
    for boss in [mano, stumpy] {
        let mut t = Trace::default();
        assert_eq!(
            warp(&mut a.connection, boss.map, "", &mut t).await,
            boss.map
        );
        until(&mut a.connection, &mut t, |t| {
            t.field.mobs.iter().any(|&(_, mob)| mob == boss.mob)
        })
        .await;
    }
}
