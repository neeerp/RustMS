#include "Snowball.h"

#include <algorithm>
#include <chrono>
#include <cstdio>
#include <cstdlib>

namespace ms
{
	namespace snowball
	{
		namespace
		{
			int32_t entered = 0;
			Rules field;

			// The ms per step of each stage (DAT_00bd8ba4).
			constexpr int32_t STEP_TIMES[] = { 150, 200, 250, 300, 350, 400, 450, 500, 0, -500 };
			constexpr int32_t STAGES = sizeof(STEP_TIMES) / sizeof(STEP_TIMES[0]);

			// A team's name, as v83 has it (0xaf5260, 0xaf5258).
			const char* team_name(uint8_t team)
			{
				return team == 0 ? "story" : "maple";
			}

			int32_t sign(int32_t value)
			{
				return (value > 0) - (value < 0);
			}
		}

		bool Rect::contains(int32_t x, int32_t y) const
		{
			return x >= left && x < right && y >= top && y < bottom;
		}

		Rect Rules::ball(size_t lane, int32_t step) const
		{
			int32_t bx = ball_x(step);
			int32_t y = lanes[lane].y - 81;

			return { bx - 80, y - 80, bx + 80, y + 85 };
		}

		Rect Rules::snowman(size_t lane) const
		{
			int32_t y = lanes[lane].y;

			return { x - 30, y - 100, x + 30, y };
		}

		int32_t Rules::ball_x(int32_t step) const
		{
			return x0 + dx * step;
		}

		std::string nx_path(const std::string& path)
		{
			const std::string prefix = "Map/";

			return path.compare(0, prefix.size(), prefix) == 0 ? path.substr(prefix.size()) : path;
		}

		StatePacket read_state(InPacket& recv, bool first)
		{
			StatePacket packet;
			packet.state = static_cast<uint8_t>(recv.read_byte());

			for (int32_t& hp : packet.hp)
				hp = recv.read_int();

			for (size_t i = 0; i < 2; i++)
			{
				packet.steps[i] = recv.read_short();
				packet.stages[i] = static_cast<uint8_t>(recv.read_byte());
			}

			if (first)
			{
				std::array<uint16_t, 3> damages;

				for (uint16_t& damage : damages)
					damage = static_cast<uint16_t>(recv.read_short());

				packet.damages = damages;
			}

			return packet;
		}

		HitPacket read_hit(InPacket& recv)
		{
			HitPacket hit;
			hit.what = static_cast<uint8_t>(recv.read_byte());
			hit.damage = static_cast<uint16_t>(recv.read_short());
			hit.delay = static_cast<uint16_t>(recv.read_short());

			return hit;
		}

		void Hits::add(const HitPacket& hit, int64_t now)
		{
			if (hit.what < TARGETS)
				pending.push_back({ hit.what, hit.damage, now + hit.delay });
		}

		std::vector<Hit> Hits::take_due(int64_t now)
		{
			std::vector<Hit> due;
			auto later = std::stable_partition(pending.begin(), pending.end(),
											   [now](const Hit& hit) { return hit.due <= now; });

			due.assign(pending.begin(), later);
			pending.erase(pending.begin(), later);

			return due;
		}

		size_t Hits::size() const
		{
			return pending.size();
		}

		int32_t Ball::set_position(int16_t to, uint8_t pace, bool snap, const Rules& rules)
		{
			if (to < rules.xmin || to > rules.xmax)
				return 0;

			stage = pace;

			if (!snap)
			{
				pending = (to - step) * 3;

				return 0;
			}

			int32_t moved = to - step;
			pending = 0;
			time = 0;
			step = to;

			return moved;
		}

		int32_t Ball::roll(int32_t ms, const Rules& rules)
		{
			int32_t pace = step_time(stage);
			int32_t behind = sign(pending);

			if (pace == 0)
			{
				// It only catches up, a step at a time.
				int32_t next = step + behind;

				if (behind == 0 || next < rules.xmin || next > rules.xmax)
				{
					time = 0;

					return 0;
				}

				time += ms;

				if (time < CATCH_UP_TIME)
					return 0;

				step = next;
				// v83 never lets the backlog go below 0, even one behind.
				pending = std::max(pending - 3 * behind, 0);
				time -= CATCH_UP_TIME;

				return behind;
			}

			int32_t way = pace < 0 ? -1 : 1;
			int32_t next = step + way;

			if (next < rules.xmin || next > rules.xmax)
			{
				time = 0;

				return 0;
			}

			// Faster when the server's step is ahead of the way it rolls,
			// slower when behind.
			int32_t wait = ((3 - way * behind) * std::abs(pace)) / 3;
			time += ms;

			if (time < wait)
				return 0;

			pending = std::max(pending - behind, 0);
			step = next;
			time -= wait;

			return way;
		}

		int32_t step_time(int32_t stage)
		{
			return stage >= 0 && stage < STAGES ? STEP_TIMES[stage] : 0;
		}

		bool snaps(bool first, uint8_t state)
		{
			return first || state != 1;
		}

		int16_t first_hit(const Rules& rules, const std::array<int32_t, 2>& steps, const coconut::Box& swing)
		{
			std::vector<coconut::Box> boxes;

			for (size_t lane = 0; lane < 2; lane++)
			{
				Rect ball = rules.ball(lane, steps[lane]);
				boxes.push_back({ ball.left, ball.top, ball.right, ball.bottom });
			}

			for (size_t lane = 0; lane < 2; lane++)
			{
				Rect snowman = rules.snowman(lane);
				boxes.push_back({ snowman.left, snowman.top, snowman.right, snowman.bottom });
			}

			return coconut::first_hit(boxes, swing);
		}

		uint16_t damage(uint8_t target, const std::array<uint16_t, 3>& damages, int32_t roll)
		{
			if (target < SNOWMAN_0)
				return damages[0];

			if (roll < 20)
				return 0;

			return roll >= 90 ? damages[2] : damages[1];
		}

		int32_t touch_damage(int32_t roll)
		{
			return 30 + roll % 11;
		}

		std::optional<std::string> message(uint8_t team, uint8_t line)
		{
			char text[128];

			switch (line)
			{
			case 1:
			case 2:
			case 3:
				std::snprintf(text, sizeof(text), "%s Team's snowball has passed the %d stage.", team_name(team),
							  line);
				break;
			case 4:
				std::snprintf(text, sizeof(text),
							  "%s Team is attacking the snowman, stopping the progress of %s Team's snowball.",
							  team_name(team == 0 ? 1 : 0), team_name(team));
				break;
			case 5:
				std::snprintf(text, sizeof(text), "%s Team's snowball is moving again.", team_name(team));
				break;
			default:
				return std::nullopt;
			}

			return std::string(text);
		}

		int32_t hp_width(int32_t hp)
		{
			return hp < 1 ? 46 : hp * 46 / 100;
		}

		float fade_opacity(int64_t elapsed)
		{
			float left = 1.0f - static_cast<float>(elapsed) / FADE_TIME;

			return std::clamp(left, 0.0f, 1.0f);
		}

		void enter_field(int32_t fieldtype, const Rules& rules)
		{
			entered = fieldtype;
			field = rules;
			state() = State();
		}

		bool on_field()
		{
			return entered == FIELD_TYPE;
		}

		const Rules& rules()
		{
			return field;
		}

		State& state()
		{
			static State current;

			return current;
		}

		void apply(const StatePacket& packet, int64_t now)
		{
			State& current = state();
			bool snap = snaps(current.state == -1, packet.state);

			current.state = packet.state;
			current.hp = packet.hp;

			if (packet.damages)
				current.damages = *packet.damages;

			for (size_t i = 0; i < 2; i++)
				current.balls[i].set_position(packet.steps[i], packet.stages[i], snap, field);

			// Every one of these fades the ball again (A 574068).
			if (packet.state == 2 || packet.state == 3)
				current.fades[packet.state - 2] = std::max<int64_t>(now, 1);
		}

		int64_t now()
		{
			auto since = std::chrono::steady_clock::now().time_since_epoch();

			return std::chrono::duration_cast<std::chrono::milliseconds>(since).count();
		}
	}
}
