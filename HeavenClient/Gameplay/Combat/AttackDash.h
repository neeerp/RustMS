#pragma once

#include <cstdint>
#include <cstdlib>
#include <deque>
#include <optional>
#include <utility>
#include <vector>

namespace ms
{
	// The attacks that carry the player along the floor once they hit: a list of steps, each
	// moving the player every tick for its time (FUN_00950921, played by 0x94e6d3).
	namespace attack_dash
	{
		// How often a step moves the player, in ms.
		constexpr int32_t TICK = 30;

		// The path element that marks the start of a course (0x952f1b), and the one that marks
		// each of the Aran's steps instead (0x94e83d).
		constexpr uint8_t RUSH_MOVE = 9;
		constexpr uint8_t ARAN_MOVE = 0x15;

		constexpr int32_t RUSH_HERO = 1121006;
		constexpr int32_t RUSH_PALADIN = 1221007;
		constexpr int32_t RUSH_DK = 1321003;
		constexpr int32_t BACKSPIN_BLOW = 5101002;
		constexpr int32_t CORKSCREW_BLOW = 5101004;
		constexpr int32_t CORKSCREW_BLOW_TB = 15101003;
		constexpr int32_t BARRAGE = 5121007;
		constexpr int32_t BARRAGE_TB = 15111004;
		constexpr int32_t DOUBLE_SWING_NOBLE = 20000014;
		constexpr int32_t TRIPLE_SWING_NOBLE = 20000015;
		constexpr int32_t DOUBLE_SWING = 21000002;
		constexpr int32_t TRIPLE_SWING = 21100001;
		constexpr int32_t FINAL_CHARGE = 21100002;
		constexpr int32_t FULL_SWING_1 = 21110007;
		constexpr int32_t FULL_SWING_2 = 21110008;
		constexpr int32_t OVER_SWING_1 = 21120009;
		constexpr int32_t OVER_SWING_2 = 21120010;

		enum class Kind : uint8_t
		{
			// The same distance every tick.
			STEADY,
			// Four fifths of the last distance every tick.
			FADING
		};

		struct Step
		{
			Kind kind;
			int32_t value;
			int32_t time;
		};

		// Where a tick takes the player: `x` along, onto a floor within `reach` above or below.
		struct Move
		{
			int32_t x;
			int32_t reach;
		};

		// The Aran's attacks mark their steps in the path, and only move if the first mob isn't
		// in the air from a toss (0x953731).
		inline bool marks_path(int32_t skill)
		{
			switch (skill)
			{
				case DOUBLE_SWING_NOBLE:
				case TRIPLE_SWING_NOBLE:
				case DOUBLE_SWING:
				case TRIPLE_SWING:
				case FINAL_CHARGE:
				case FULL_SWING_1:
				case FULL_SWING_2:
				case OVER_SWING_1:
				case OVER_SWING_2:
					return true;
				default:
					return false;
			}
		}

		// The steps of `skill` from a player facing `left`, none for any other attack (0x952e0d
		// to 0x9534fb). `area` is the left of the skill's lt/rb, and `mob_moves` whether the first
		// mob hit can move, or the shove has nothing to follow.
		inline std::vector<Step> course(int32_t skill, bool left, int32_t area, bool mob_moves)
		{
			constexpr Kind STEADY = Kind::STEADY;
			constexpr Kind FADING = Kind::FADING;

			int32_t ahead = left ? -1 : 1;
			int32_t rush = std::abs(area / 8) * ahead;
			int32_t shove = mob_moves ? 15 * ahead : 0;

			switch (skill)
			{
				case RUSH_HERO:
				case RUSH_PALADIN:
				case RUSH_DK:
					return { { STEADY, rush, 90 }, { FADING, rush, 180 } };
				case FINAL_CHARGE:
					return { { STEADY, 0, 60 }, { STEADY, rush, 90 }, { FADING, rush, 180 } };
				case CORKSCREW_BLOW:
				case CORKSCREW_BLOW_TB:
					return { { STEADY, 109 * ahead, 30 }, { STEADY, 0, 90 }, { STEADY, 9 * ahead, 30 },
							 { STEADY, 0, 90 }, { STEADY, 6 * ahead, 30 }, { STEADY, 0, 90 },
							 { STEADY, 4 * ahead, 30 }, { STEADY, 0, 90 }, { STEADY, 2 * ahead, 30 },
							 { STEADY, 0, 90 }, { STEADY, 0, 1200 } };
				case BACKSPIN_BLOW:
					return { { STEADY, 0, 480 }, { STEADY, -120 * ahead, 30 }, { STEADY, 0, 90 },
							 { STEADY, -6 * ahead, 30 }, { STEADY, 0, 90 }, { STEADY, -2 * ahead, 30 },
							 { STEADY, 0, 90 }, { STEADY, -1 * ahead, 30 }, { STEADY, 0, 90 } };
				case BARRAGE:
				case BARRAGE_TB:
					return { { STEADY, 0, 720 }, { STEADY, mob_moves ? 14 * ahead : 0, 30 }, { STEADY, 0, 1230 } };
				case DOUBLE_SWING:
				case DOUBLE_SWING_NOBLE:
					return { { STEADY, 0, 240 }, { STEADY, shove, 30 } };
				case TRIPLE_SWING:
				case TRIPLE_SWING_NOBLE:
				case FULL_SWING_2:
				case OVER_SWING_2:
					return { { STEADY, 0, 150 }, { STEADY, shove, 30 } };
				case FULL_SWING_1:
				case OVER_SWING_1:
					return { { STEADY, shove, 30 } };
				default:
					return {};
			}
		}

		// How long after hitting `skill` can't go again, in ms; 0 for one that isn't held back
		// (0x950d05).
		inline int64_t lock(int32_t skill)
		{
			switch (skill)
			{
				case RUSH_HERO:
				case RUSH_PALADIN:
				case RUSH_DK:
					return 1770;
				case CORKSCREW_BLOW:
				case CORKSCREW_BLOW_TB:
					return 1450;
				case BACKSPIN_BLOW:
					return 1960;
				default:
					return 0;
			}
		}

		// The steps being played.
		class Course
		{
		public:
			void start(int32_t id, std::vector<Step> course)
			{
				skill_id = id;
				steps.assign(course.begin(), course.end());
				active = true;
			}

			void stop()
			{
				active = false;
			}

			// Whether steps are left, which keeps the player from being hit (0x958230).
			bool running() const
			{
				return active && !steps.empty();
			}

			int32_t skill() const
			{
				return skill_id;
			}

			// Plays one tick; nothing if it only waits. A fading step goes first and a step
			// reaches at least 30 px, unless it fades, or by the shorter way to the left.
			std::optional<Move> tick()
			{
				if (!active)
					return std::nullopt;

				while (!steps.empty() && steps.front().time <= 0)
					steps.pop_front();

				if (steps.empty())
					return std::nullopt;

				Step &step = steps.front();
				step.time -= TICK;

				if (step.value == 0)
					return std::nullopt;

				if (step.kind == Kind::FADING)
					step.value = static_cast<int32_t>(step.value * 0.8);

				int32_t reach = step.value < 30 && step.kind != Kind::FADING ? 30 : step.value;

				return Move{ step.value, std::abs(reach) };
			}

		private:
			std::deque<Step> steps;
			int32_t skill_id = 0;
			bool active = false;
		};
	}
}
