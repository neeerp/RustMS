#pragma once

#include <cstddef>
#include <cstdint>
#include <deque>
#include <optional>
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
		// Body Pressure, whose buff makes touching a mob hit it (0x94b49d).
		constexpr int32_t BODY_PRESSURE = 21101003;
		// A polearm's Weapon::Type, its id's first three digits.
		constexpr int32_t POLEARM = 144;

		// Combo Ability for a job.
		inline int32_t ability(int32_t job)
		{
			return job == ARAN_BEGINNER ? 20000017 : 21000000;
		}

		// Combo Critical for a job.
		inline int32_t critical(int32_t job)
		{
			return job == ARAN_BEGINNER ? 20000018 : 21110000;
		}

		// What Combo Critical adds for a combo, from its x, y and damage: y percent of rate for every ten,
		// up to x tens, and 10 more, and its damage for every ten (0x78e254).
		struct Critical
		{
			int32_t rate;
			int32_t damage;
		};

		inline Critical critical(int32_t count, int32_t x, int32_t y, int32_t damage)
		{
			int32_t tens = x < count / 10 ? x : count / 10;

			return { y * tens + 10, damage * tens };
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

		// What the combo adds to attack (0x77df48): Combo Ability's `y` for every ten, up to its `x` tens.
		inline int32_t attack(int32_t count, int32_t x, int32_t y)
		{
			if (count <= 0)
				return 0;

			return (x < count / 10 ? x : count / 10) * y;
		}

		// How hard a hit must be for a mob that isn't a boss to flinch, as the player's Smart Knockback
		// keeps `smart` percent of the mob's `pushed` (0 without it) (FUN_00671210).
		inline int32_t pushed(int32_t pushed, int32_t smart, bool boss)
		{
			if (boss || smart <= 0)
				return pushed;

			return static_cast<int32_t>(static_cast<double>(pushed * smart) * 0.01);
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

		// The combo commands (v83ref report b362-aran-combo-commands.md): Down, then Left or Right, then the
		// attack key uses the best of Combo Tempest, Penril and Smash the combo allows, and Down, Down let go,
		// Down, then the attack key the best of Combo Barrier and Drain. They are entries of v83's table of
		// key sequences (FUN_0074ab67), which also holds other Aran attacks not made here yet.
		constexpr int32_t TO_THE_LEFT = 9;
		constexpr int32_t TO_THE_RIGHT = 10;
		constexpr int32_t DOWN_TWICE = 11;
		// How soon after its first key a sequence must end.
		constexpr int64_t WINDOW = 600;
		// How long a command matched waits to be used before it is dropped (vt +0x18).
		constexpr int64_t WAITS = 1000;

		// The keys the sequences are made of: the arrows and the key the attack is bound to.
		enum class Key
		{
			OTHER,
			UP,
			DOWN,
			LEFT,
			RIGHT,
			ATTACK
		};

		struct Node
		{
			Key key;
			bool down;
		};

		struct Sequence
		{
			int32_t id;
			std::vector<Node> nodes;
		};

		inline const std::vector<Sequence>& sequences()
		{
			static const std::vector<Sequence> table = {
					{ TO_THE_LEFT, { { Key::DOWN, true }, { Key::LEFT, true }, { Key::ATTACK, true } } },
					{ TO_THE_RIGHT, { { Key::DOWN, true }, { Key::RIGHT, true }, { Key::ATTACK, true } } },
					{ DOWN_TWICE, { { Key::DOWN, true }, { Key::DOWN, false }, { Key::DOWN, true }, { Key::ATTACK, true } } }
			};

			return table;
		}

		// Whether a command may begin: an Aran with a polearm, who knows Combo Smash for the first two and
		// Combo Drain for the last.
		template <typename Level>
		bool enabled(int32_t id, int32_t job, int32_t weapon_type, Level level)
		{
			bool aran = job / 100 == 21 || job == ARAN_BEGINNER;

			if (!aran || weapon_type != POLEARM)
				return false;

			return level(id == DOWN_TWICE ? 21100005 : 21100004) > 0;
		}

		// The skill a command uses for a combo, or 0 when the combo allows none of its skills (0x74d314,
		// 0x74d51a).
		template <typename Level>
		int32_t pick(int32_t id, int32_t count, Level level)
		{
			static const std::vector<int32_t> sideways = { 21120006, 21110004, 21100004 };
			static const std::vector<int32_t> down_twice = { 21120007, 21100005 };

			for (int32_t skill : id == DOWN_TWICE ? down_twice : sideways)
				if (count >= needs(skill) && level(skill) > 0)
					return skill;

			return 0;
		}

		// Which way a command turns the player before each try: Left's to the left and Right's to the right,
		// the other way round while confused. Down twice doesn't turn.
		inline std::optional<bool> faces_right(int32_t id, bool confused)
		{
			if (id == DOWN_TWICE)
				return std::nullopt;

			return (id == TO_THE_RIGHT) != confused;
		}

		// The matcher (FUN_0074d993). A key repeating its last event is ignored. Every sequence begun goes
		// on while each key matches its next one or is a key let go, and ends past its window, checked as
		// keys come, or on any other key going down. New ones begin only on a key no sequence took.
		class Commands
		{
		public:
			// The command `code`'s key going `down` completes, or 0. `may_start(id)` says which may begin.
			template <typename Enabled>
			int32_t feed(int64_t code, Key key, bool down, int64_t now, Enabled may_start)
			{
				if (fed && code == last_code && down == last_down)
					return 0;

				fed = true;
				last_code = code;
				last_down = down;

				const std::vector<Sequence>& table = sequences();
				std::vector<Active> going;
				int32_t done = 0;

				for (Active a : active)
				{
					const Sequence& sequence = table[a.sequence];

					if (now > a.start + WINDOW)
						continue;

					const Node& next = sequence.nodes[a.next];

					if (next.key == key && next.down == down)
					{
						if (++a.next == sequence.nodes.size())
						{
							done = sequence.id;
							continue;
						}

						going.push_back(a);
					}
					else if (!down)
					{
						going.push_back(a);
					}
				}

				if (done != 0)
				{
					active.clear();
					return done;
				}

				if (going.empty())
				{
					for (size_t i = 0; i < table.size(); i++)
					{
						const Node& first = table[i].nodes.front();

						if (first.key == key && first.down == down && may_start(table[i].id))
							going.push_back({ i, 1, now });
					}
				}

				active = going;

				return 0;
			}

			void clear()
			{
				active.clear();
			}

		private:
			struct Active
			{
				size_t sequence;
				size_t next;
				int64_t start;
			};

			std::vector<Active> active;
			bool fed = false;
			int64_t last_code = 0;
			bool last_down = false;
		};

		// The commands matched, waiting to be used one by one (mgr+0x3c). While one waits the keys that act
		// are dropped (FUN_0094f2ed), which keeps the last key from attacking too.
		class Queue
		{
		public:
			void push(int32_t id, int64_t now)
			{
				queued.push_back({ id, now });
			}

			bool empty() const
			{
				return queued.empty();
			}

			// The command to try now, once those that waited too long are dropped.
			std::optional<int32_t> due(int64_t now)
			{
				while (!queued.empty() && now >= queued.front().at + WAITS)
					queued.pop_front();

				if (queued.empty())
					return std::nullopt;

				return queued.front().id;
			}

			void pop()
			{
				if (!queued.empty())
					queued.pop_front();
			}

			void clear()
			{
				queued.clear();
			}

		private:
			struct Queued
			{
				int32_t id;
				int64_t at;
			};

			std::deque<Queued> queued;
		};
	}
}
