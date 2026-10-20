#pragma once

#include "../Character/SkillMacros.h"

#include <cstdint>
#include <deque>
#include <optional>

namespace ms
{
	// Running a skill macro (report 206 3): its key queues the skills, which
	// go through the normal skill use one try per 30 ms tick, each until it
	// goes off. A skill still failing after 4 s drops the rest.
	namespace skill_macro
	{
		// v83 tries the next skill once per CUserLocal::Update.
		constexpr int32_t TICK = 30;
		// How long the skill at the head may keep failing (FUN_0063196b).
		constexpr int32_t GIVE_UP = 4000;

		// A skill no macro runs: none, a passive one (digit 0) or a digit-9 one.
		inline bool skipped(int32_t skill)
		{
			int32_t digit = skill / 1000 % 10;

			return skill == 0 || digit == 0 || digit == 9;
		}

		struct Step
		{
			int32_t skill;
			// The macro's place in the list.
			int32_t macro;
			// Says the macro's name when it goes off.
			bool first;
		};

		class Runner
		{
		public:
			// Queue the macro's skills, unless one is running (v83 FUN_00631b81).
			// Skills with a cooldown at any level are left out, but only the
			// first skill not skipped may say the name, even if it has one.
			template<typename HasCooltime>
			void start(const SkillMacros& macros, int32_t index, HasCooltime has_cooltime)
			{
				if (running() || index < 0 || static_cast<size_t>(index) >= macros.size())
					return;

				bool first = true;

				for (size_t slot = 0; slot < 3; slot++)
				{
					int32_t skill = macros.skill(index, slot);

					if (skipped(skill))
						continue;

					bool says = first;
					first = false;

					if (!has_cooltime(skill))
						queue.push_back({ skill, index, says });
				}
			}

			// Drop the rest (v83 FUN_00631a80).
			void clear()
			{
				queue.clear();
				waited = 0;
			}

			bool running() const
			{
				return !queue.empty();
			}

			// The skill to try, once a tick has passed with one queued.
			std::optional<Step> due(int32_t elapsed)
			{
				since += elapsed;

				if (since < TICK)
					return std::nullopt;

				since %= TICK;

				if (queue.empty())
					return std::nullopt;

				return queue.front();
			}

			// The skill `due` gave went off, or didn't.
			void tried(bool went_off)
			{
				if (went_off)
				{
					queue.pop_front();
					waited = 0;
				}
				else if ((waited += TICK) > GIVE_UP)
				{
					clear();
				}
			}

		private:
			std::deque<Step> queue;
			int32_t waited = 0;
			int32_t since = 0;
		};
	}
}
