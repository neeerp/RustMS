#pragma once

#include <algorithm>
#include <cstdint>
#include <vector>

namespace ms
{
	// Friendly mobs (Mob.wz info/damagedByMob), such as Henesys's Moon
	// Bunny: players can't hurt them, but other mobs that run into them can,
	// which one client on the map reports (CMob::Update, FUN_006675a8).
	namespace friendly
	{
		// The least time between two reports from this client, over every
		// mob on the map, from when the friendly came (0x667efe, 0x679565).
		constexpr int64_t REPORT_EVERY = 1500;

		// The ms between the blink's steps, one per v83 update (0x6686d5).
		constexpr int32_t BLINK_STEP = 30;

		// Whether a friendly hurt by mobs (`damagedby`, its
		// damagedBySelectedMob list, empty for any) is hurt by mob `id`
		// (FUN_00670bc0).
		inline bool hurt_by(bool friendly, const std::vector<int32_t> &damagedby, int32_t id)
		{
			if (!friendly)
				return false;

			return damagedby.empty() || std::find(damagedby.begin(), damagedby.end(), id) != damagedby.end();
		}

		// Whether this client reports hits on the friendly: its party's
		// leader, or anyone with no other players on the map, so that one
		// client does (0x667ed5).
		inline bool reports(bool leader, size_t others)
		{
			return leader || others == 0;
		}

		// Whether a report may go `now`, the last having gone at `last`.
		inline bool may_report(int64_t now, int64_t last)
		{
			return now - last >= REPORT_EVERY;
		}

		// Whether two bodies, each left, top, right and bottom, meet: they
		// share more than an edge (IntersectRect, 0x667f0f).
		inline bool meet(int16_t l1, int16_t t1, int16_t r1, int16_t b1, int16_t l2, int16_t t2, int16_t r2, int16_t b2)
		{
			return l1 < r2 && l2 < r1 && t1 < b2 && t2 < b1;
		}

		// Its gauge's fill, as DAMAGE_MONSTER gives its HP (0x66c714).
		inline int16_t percent(int32_t hp, int32_t maxhp)
		{
			if (maxhp <= 0)
				return 0;

			int64_t pct = static_cast<int64_t>(hp) * 100 / maxhp;

			return static_cast<int16_t>(std::clamp<int64_t>(pct, 0, 100));
		}

		// Whether the friendly shows grey `elapsed` ms into a hit: two
		// steps grey, two not (0x6686d5).
		inline bool grey(int32_t elapsed)
		{
			return (elapsed / BLINK_STEP & 3) < 2;
		}
	}
}
