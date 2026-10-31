#pragma once

#include "../../Character/Look/Stance.h"

#include <algorithm>
#include <cstddef>
#include <cstdint>
#include <deque>
#include <optional>
#include <string>
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

		// The key sequences (v83ref reports b362-aran-combo-commands.md, b364-aran-key-sequences.md), v83's table
		// of them (FUN_0074ab67) less the Dash's double tap, which is made apart (dash::Taps). The Aran's: two
		// presses of the attack key Double Swing, and a third Triple Swing; Left or Right, then the attack key,
		// Final Charge; Down, then it, Final Blow; Up, then it, Final Toss; Up, Up let go, Up and it, Rolling
		// Spin; Down, then Left or Right, then it, the best of Combo Tempest, Penril and Smash the combo allows;
		// Down, Down let go, Down and it, the best of Combo Barrier and Drain; a double tap on Left or Right,
		// Combat Step. Anyone's: Down, Up and it, Rage of Pharaoh in the Pyramid.
		constexpr int32_t DOUBLE_SWING = 2;
		constexpr int32_t TRIPLE_SWING = 3;
		constexpr int32_t CHARGE_LEFT = 4;
		constexpr int32_t CHARGE_RIGHT = 5;
		constexpr int32_t FINAL_BLOW = 6;
		constexpr int32_t FINAL_TOSS = 7;
		constexpr int32_t ROLLING_SPIN = 8;
		constexpr int32_t TO_THE_LEFT = 9;
		constexpr int32_t TO_THE_RIGHT = 10;
		constexpr int32_t DOWN_TWICE = 11;
		constexpr int32_t STEP_LEFT = 12;
		constexpr int32_t STEP_RIGHT = 13;
		constexpr int32_t RAGE = 14;
		// How long a command matched waits to be used before it is dropped (vt +0x18): Final Charge's, Blow's
		// and Toss's two seconds, the rest one.
		constexpr int64_t WAITS = 1000;

		inline int64_t waits(int32_t id)
		{
			return id >= CHARGE_LEFT && id <= FINAL_TOSS ? 2 * WAITS : WAITS;
		}

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

		// A key going down or up. Once it matches, the sequence has `timeout` more ms, counted from its first
		// key, for the keys after it (0 adds none, and a sequence whose first adds none has no end), and the
		// next key must match the next node, unless it is this key going down again or, if `loose`, any key
		// let go. The `command` node is the one that makes the sequence's command.
		struct Node
		{
			Key key;
			bool down;
			int64_t timeout;
			bool loose;
			bool command;
		};

		struct Sequence
		{
			int32_t id;
			// Whether it begins on its own, or only after another ends.
			bool root;
			std::vector<Node> nodes;
			// The sequences that begin as its last node is reached.
			std::vector<int32_t> then;
		};

		inline const std::vector<Sequence>& sequences()
		{
			auto press = [](Key key, int64_t timeout = 0, bool loose = false)
			{
				return Node{ key, true, timeout, loose, false };
			};
			auto release = [](Key key)
			{
				return Node{ key, false, 0, false, false };
			};
			auto command = [](Key key, int64_t timeout = 0, bool loose = false)
			{
				return Node{ key, true, timeout, loose, true };
			};
			auto tap = [&](int32_t id, Key key)
			{
				return Sequence{ id, true, { press(key, 300), release(key), command(key), release(key) }, {} };
			};

			static const std::vector<Sequence> table = {
					{ DOUBLE_SWING,
					  true,
					  { press(Key::ATTACK, 480), release(Key::ATTACK), command(Key::ATTACK, 420), release(Key::ATTACK) },
					  { TRIPLE_SWING } },
					{ TRIPLE_SWING, false, { command(Key::ATTACK, 660), release(Key::ATTACK) }, {} },
					{ CHARGE_LEFT, true, { press(Key::LEFT, 600, true), command(Key::ATTACK) }, {} },
					{ CHARGE_RIGHT, true, { press(Key::RIGHT, 600, true), command(Key::ATTACK) }, {} },
					{ FINAL_BLOW, true, { press(Key::DOWN, 300, true), command(Key::ATTACK) }, {} },
					{ FINAL_TOSS, true, { press(Key::UP, 300, true), command(Key::ATTACK) }, {} },
					{ ROLLING_SPIN,
					  true,
					  { press(Key::UP, 700), release(Key::UP), press(Key::UP, 0, true), command(Key::ATTACK, 0, true) },
					  {} },
					{ TO_THE_LEFT,
					  true,
					  { press(Key::DOWN, 600, true), press(Key::LEFT, 0, true), command(Key::ATTACK, 0, true) },
					  {} },
					{ TO_THE_RIGHT,
					  true,
					  { press(Key::DOWN, 600, true), press(Key::RIGHT, 0, true), command(Key::ATTACK, 0, true) },
					  {} },
					{ DOWN_TWICE,
					  true,
					  { press(Key::DOWN, 600), release(Key::DOWN), press(Key::DOWN, 0, true),
						command(Key::ATTACK, 0, true) },
					  {} },
					tap(STEP_LEFT, Key::LEFT),
					tap(STEP_RIGHT, Key::RIGHT),
					{ RAGE, true, { press(Key::DOWN, 300, true), press(Key::UP, 0, true), command(Key::ATTACK, 0, true) },
					  {} }
			};

			return table;
		}

		// The skill a command uses whose level it needs, for a job: the Legend's own copies of Double and
		// Triple Swing and Final Blow, or Rage of Pharaoh's copy for the job's family; 0 for the combo commands,
		// whose skill the combo picks.
		inline int32_t skill(int32_t id, int32_t job)
		{
			bool legend = job == ARAN_BEGINNER;

			switch (id)
			{
				case DOUBLE_SWING:
					return legend ? 20000014 : 21000002;
				case TRIPLE_SWING:
					return legend ? 20000015 : 21100001;
				case CHARGE_LEFT:
				case CHARGE_RIGHT:
					return 21100002;
				case FINAL_BLOW:
					return legend ? 20000016 : 21120005;
				case FINAL_TOSS:
					return 21110003;
				case ROLLING_SPIN:
					return 21110006;
				case STEP_LEFT:
				case STEP_RIGHT:
					return 21001001;
				case RAGE:
				{
					int32_t family = job / 100 == 22 || job == 2001 ? 2001 : job / 1000 * 1000;

					return family * 10000 + 1020;
				}
				default:
					return 0;
			}
		}

		// Whether a sequence may begin: an Aran (Combat Step's with any weapon, the rest's with a polearm) who
		// knows its skill, either of the Legend's copy and the Aran's for the swings and Final Blow, and Combo
		// Smash for the sideways commands and Combo Drain for down twice. Rage of Pharaoh's always may.
		template <typename Level>
		bool enabled(int32_t id, int32_t job, int32_t weapon_type, Level level)
		{
			if (id == RAGE)
				return true;

			bool aran = job / 100 == 21 || job == ARAN_BEGINNER;

			if (!aran)
				return false;

			switch (id)
			{
				case STEP_LEFT:
				case STEP_RIGHT:
					return level(21001001) > 0;
				default:
					break;
			}

			if (weapon_type != POLEARM)
				return false;

			switch (id)
			{
				case TO_THE_LEFT:
				case TO_THE_RIGHT:
					return level(21100004) > 0;
				case DOWN_TWICE:
					return level(21100005) > 0;
				case DOUBLE_SWING:
				case TRIPLE_SWING:
				case FINAL_BLOW:
					return level(skill(id, 2100)) > 0 || level(skill(id, ARAN_BEGINNER)) > 0;
				default:
					return level(skill(id, job)) > 0;
			}
		}

		// The skill a combo command uses for a combo, or 0 when the combo allows none of its skills (0x74d314,
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
		// the other way round while confused. The rest don't turn.
		inline std::optional<bool> faces_right(int32_t id, bool confused)
		{
			switch (id)
			{
				case CHARGE_LEFT:
				case TO_THE_LEFT:
				case STEP_LEFT:
					return confused;
				case CHARGE_RIGHT:
				case TO_THE_RIGHT:
				case STEP_RIGHT:
					return !confused;
				default:
					return std::nullopt;
			}
		}

		// Full Swing and Over Swing make Double and Triple Swing cast their own hidden skills, at their own
		// level, Over Swing first; the Legend's copies stay as they are (FUN_00966f7a, report b365).
		constexpr int32_t FULL_SWING = 21110002;
		constexpr int32_t OVER_SWING = 21120002;

		inline int32_t swing_cast(int32_t skill_id, int32_t over_level, int32_t full_level)
		{
			bool triple = skill_id == 21100001;

			if (skill_id != 21000002 && !triple)
				return skill_id;

			if (over_level > 0)
				return triple ? 21120010 : 21120009;

			if (full_level > 0)
				return triple ? 21110008 : 21110007;

			return skill_id;
		}

		// How many mobs an attack with no skill hits: an Aran's with a polearm two more than a twentieth of
		// their level, anyone else's one (0x9514c3, report b365).
		inline int32_t plain_mob_count(int32_t job, int32_t weapon_type, int32_t level)
		{
			if ((job / 100 != 21 && job != ARAN_BEGINNER) || weapon_type != POLEARM)
				return 1;

			return level / 20 + 2;
		}

		// The skill whose level a hidden swing takes, or 0 for any other skill (FUN_007616f6, report b365).
		inline int32_t swing_owner(int32_t skill_id)
		{
			switch (skill_id)
			{
				case 21110007:
				case 21110008:
					return FULL_SWING;
				case 21120009:
				case 21120010:
					return OVER_SWING;
				default:
					return 0;
			}
		}

		// What the player's body is doing, for the swings: an attack with the polearm, a Double Swing (or its
		// Full or Over Swing), a Triple Swing (likewise), or anything else.
		enum class Doing
		{
			OTHER,
			SWING,
			DOUBLE,
			TRIPLE
		};

		inline Doing doing(const std::string& action)
		{
			if (action == "swingT2PoleArm" || action == "swingP1PoleArm" || action == "swingP2PoleArm")
				return Doing::SWING;

			if (action == "doubleSwing" || action == "fullSwingDouble" || action == "overSwingDouble")
				return Doing::DOUBLE;

			if (action == "tripleSwing" || action == "fullSwingTriple" || action == "overSwingTriple")
				return Doing::TRIPLE;

			return Doing::OTHER;
		}

		// An attack with no skill by an Aran with a polearm swings it one of the polearm's own three ways, chosen
		// at random, where anyone else's takes the weapon's usual stances (0x950ec9, report b365). Each comes
		// with the plain swing sent and shown to others in its place.
		struct Swing
		{
			const char* action;
			Stance::Id stance;
		};

		inline std::optional<Swing> polearm_swing(int32_t job, int32_t weapon_type, int32_t roll)
		{
			if ((job / 100 != 21 && job != ARAN_BEGINNER) || weapon_type != POLEARM)
				return std::nullopt;

			static const Swing swings[3] = {
				{ "swingT2PoleArm", Stance::Id::SWINGT2 },
				{ "swingP1PoleArm", Stance::Id::SWINGP1 },
				{ "swingP2PoleArm", Stance::Id::SWINGP2 }
			};

			return swings[roll % 3];
		}

		// That swing then holds off the next attack with no skill for the length of its action, every frame's delay
		// whatever its sign and whatever the attack speed, and 400 ms more (0x951936, report b365).
		constexpr int64_t SWING_REST = 400;

		inline int64_t polearm_swing_wait(const std::vector<uint16_t> &delays)
		{
			int64_t wait = SWING_REST;

			for (uint16_t delay : delays)
				wait += delay;

			return wait;
		}

		// What a command does as its sequence matches, given whether the player may use it (its skill learned,
		// or for Rage of Pharaoh a use left, or for a combo command a skill the combo allows), what the body is
		// doing and the command waiting last. Double Swing waits only on an attack with the polearm and before
		// any other; Triple Swing on a Double Swing, or after one waiting; Final Charge, Blow and Toss likewise
		// on or after a Triple Swing; the rest always (0x74c7c3 to 0x74d841). Final Blow, Final Toss, Rolling
		// Spin, Combat Step and Rage of Pharaoh take the key, which then does nothing else.
		struct Outcome
		{
			bool waits;
			bool takes;
		};

		inline Outcome outcome(int32_t id, bool usable, Doing body, std::optional<int32_t> last)
		{
			if (!usable)
				return { false, false };

			auto after = [&](Doing on, int32_t before)
			{
				return body == on && !last ? true : last == before;
			};

			switch (id)
			{
				case DOUBLE_SWING:
					return { body == Doing::SWING && !last, false };
				case TRIPLE_SWING:
					return { after(Doing::DOUBLE, DOUBLE_SWING), false };
				case CHARGE_LEFT:
				case CHARGE_RIGHT:
					return { after(Doing::TRIPLE, TRIPLE_SWING), false };
				case FINAL_BLOW:
				case FINAL_TOSS:
					return { after(Doing::TRIPLE, TRIPLE_SWING), true };
				case TO_THE_LEFT:
				case TO_THE_RIGHT:
				case DOWN_TWICE:
					return { true, false };
				default:
					return { true, true };
			}
		}

		// The matcher (FUN_0074d993). A key repeating its last event is ignored. Each sequence begun goes on
		// while its time lasts and each key matches its next node or is one its last node lets pass; a command
		// node matched makes its command. The sequences that follow one begin as it reaches its last node, and
		// replace all the others; otherwise those that may begin do on their first key, unless going already.
		class Commands
		{
		public:
			// Feeds `code`'s key going `down`. `enabled(id)` says which sequences may begin, and
			// `make(id)` makes a command, saying whether it takes the key. Returns whether one did.
			template <typename Enabled, typename Make>
			bool feed(int64_t code, Key key, bool down, int64_t now, Enabled enabled, Make make)
			{
				if (fed && code == last_code && down == last_down)
					return false;

				fed = true;
				last_code = code;
				last_down = down;

				const std::vector<Sequence>& table = sequences();
				std::vector<Active> kept;
				std::vector<Active> following;
				std::vector<int32_t> passed;
				bool taken = false;

				auto matches = [&](const Node& node)
				{
					return node.key == key && node.down == down;
				};

				for (Active a : active)
				{
					const Sequence& sequence = table[a.sequence];
					bool passes = false;

					if (a.next > 0)
					{
						const Node& node = sequence.nodes[a.next - 1];
						passes = (!down && node.loose) || (down && matches(node));
					}

					if (passes)
						passed.push_back(sequence.id);

					if (a.timeout != 0 && now > a.start + a.timeout)
						continue;

					if (passes)
					{
						kept.push_back(a);
						continue;
					}

					if (a.next >= sequence.nodes.size())
						continue;

					const Node& node = sequence.nodes[a.next];

					if (a.next + 1 == sequence.nodes.size() && following.empty())
						for (int32_t id : sequence.then)
							if (enabled(id))
								following.push_back({ index(id), 0, a.start, a.timeout + node.timeout });

					a.timeout += node.timeout;
					a.next++;

					if (!matches(node))
						continue;

					if (node.command && make(sequence.id))
						taken = true;

					kept.push_back(a);
				}

				if (!following.empty())
				{
					active = following;

					return taken;
				}

				active = kept;

				for (size_t i = 0; i < table.size(); i++)
				{
					const Sequence& sequence = table[i];
					const Node& first = sequence.nodes.front();
					bool going = std::find(passed.begin(), passed.end(), sequence.id) != passed.end();

					if (sequence.root && !going && matches(first) && enabled(sequence.id))
						active.push_back({ i, 1, now, first.timeout });
				}

				return taken;
			}

			void clear()
			{
				active.clear();
			}

		private:
			// A sequence going: the node it waits on next, when its first key came, and how long it has.
			struct Active
			{
				size_t sequence;
				size_t next;
				int64_t start;
				int64_t timeout;
			};

			static size_t index(int32_t id)
			{
				const std::vector<Sequence>& table = sequences();

				for (size_t i = 0; i < table.size(); i++)
					if (table[i].id == id)
						return i;

				return 0;
			}

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
				while (!queued.empty() && now >= queued.front().at + waits(queued.front().id))
					queued.pop_front();

				if (queued.empty())
					return std::nullopt;

				return queued.front().id;
			}

			// The command waiting last.
			std::optional<int32_t> last() const
			{
				if (queued.empty())
					return std::nullopt;

				return queued.back().id;
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
