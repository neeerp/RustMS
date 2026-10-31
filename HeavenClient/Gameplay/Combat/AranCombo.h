#pragma once

#include <cstdint>
#include <vector>

namespace ms
{
	// The Aran's combo: the hits their polearm lands while they know Combo Ability (v83ref reports
	// b360-aran-combo.md, b361-aran-combo-hud.md). v83 counts nothing itself: it tells the server of each hit
	// (0xA3), shows the count the server answers (SHOW_COMBO), and forgets it 3 seconds after the last one.
	namespace aran_combo
	{
		// How long a count lasts after the server last showed it, and the HUD with it.
		constexpr int64_t IDLE = 3000;

		constexpr int32_t ARAN_BEGINNER = 2000;
		// A polearm's Weapon::Type, its id's first three digits.
		constexpr int32_t POLEARM = 144;

		// Combo Ability for a job.
		inline int32_t ability(int32_t job)
		{
			return job == ARAN_BEGINNER ? 20000017 : 21000000;
		}

		// The combo a skill needs, which using it spends, or 0: Combo Smash and Combo Drain 30, Combo Penril
		// 100, Combo Tempest and Combo Barrier 200.
		inline int32_t needs(int32_t skill)
		{
			switch (skill)
			{
				case 21100004:
				case 21100005:
					return 30;
				case 21110004:
					return 100;
				case 21120006:
				case 21120007:
					return 200;
				default:
					return 0;
			}
		}

		// Whether a landed damage line counts: the player's own, with damage, Combo Ability learned and a
		// polearm out.
		inline bool counts(bool own, int32_t damage, int32_t ability_level, int32_t weapon_type)
		{
			return own && damage > 0 && ability_level > 0 && weapon_type == POLEARM;
		}

		// Whether the count has lasted its time (0x94bdba).
		inline bool idle(int32_t count, int64_t shown, int64_t now)
		{
			return count > 0 && now > shown + IDLE;
		}

		// The HUD's colour for a count: under 30, under 100, under 200, and the rest.
		inline int32_t tier(int32_t count)
		{
			if (count < 30)
				return 0;

			if (count < 100)
				return 1;

			if (count < 200)
				return 2;

			return 3;
		}

		// The HUD's digits, the ones first, five at most.
		inline std::vector<int32_t> digits(int32_t count)
		{
			std::vector<int32_t> shown;

			for (; count > 0 && shown.size() < 5; count /= 10)
				shown.push_back(count % 10);

			return shown;
		}

		// How high digit `i` of `n` sits: the first 165, and the rest alternating with 169 towards the ones.
		inline int16_t digit_y(size_t i, size_t n)
		{
			return i % 2 == n % 2 ? 169 : 165;
		}

		// How far a new count's digit `i` bounces down: the lowest digit that isn't 0, and the zeros under it,
		// 10, the rest 4, and none from 200 on (0x960a24).
		inline int32_t bounce_height(int32_t count, size_t i)
		{
			std::vector<int32_t> shown = digits(count);

			if (shown.empty() || tier(count) == 3)
				return 0;

			size_t lowest = 0;

			while (lowest + 1 < shown.size() && shown[lowest] == 0)
				lowest++;

			return i <= lowest ? 10 : 4;
		}

		// Where a bounce of `height` is `elapsed` ms in: down in 50 ms and back up in 250, along the Hermite
		// curve of its keys, whose slopes are 10, 10 and 0.
		inline int32_t bounce(int32_t height, int64_t elapsed)
		{
			auto hermite = [](double p0, double m0, double p1, double m1, double t)
			{
				double t2 = t * t;
				double t3 = t2 * t;

				return (2 * t3 - 3 * t2 + 1) * p0 + (t3 - 2 * t2 + t) * m0 + (-2 * t3 + 3 * t2) * p1 + (t3 - t2) * m1;
			};

			if (height == 0 || elapsed < 0 || elapsed >= 300)
				return 0;

			if (elapsed < 50)
				return static_cast<int32_t>(hermite(0.0, 10.0, height, 10.0, elapsed / 50.0));

			return static_cast<int32_t>(hermite(height, 10.0, 0.0, 0.0, (elapsed - 50) / 250.0));
		}

		// The ComboCommand a count announces, by its number (0 for none), and the skills its two lines wait
		// on: at 30 Combo Smash's and Combo Drain's, at 100 Combo Penril's and Combo Drain's, and at 200
		// Combo Tempest's and Combo Barrier's (0x960bc3).
		struct Command
		{
			int32_t index;
			int32_t first;
			int32_t second;
		};

		inline Command command(int32_t count)
		{
			switch (count)
			{
				case 30:
					return { 1, 21100004, 21100005 };
				case 100:
					return { 2, 21110004, 21100005 };
				case 200:
					return { 3, 21120006, 21120007 };
				default:
					return { 0, 0, 0 };
			}
		}
	}
}
