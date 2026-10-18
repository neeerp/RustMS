#pragma once

#include <array>
#include <cstddef>
#include <cstdint>
#include <utility>

namespace ms
{
	namespace pets
	{
		// A user's pets fill three slots that close up (FUN_0092eb37). One
		// put in at `index` moves those from there up a slot, and the last
		// falls off.
		template <typename T>
		void insert(std::array<T, 3> &slots, size_t index, T pet)
		{
			if (index >= slots.size())
				return;

			for (size_t i = slots.size() - 1; i > index; i--)
				slots[i] = std::move(slots[i - 1]);

			slots[index] = std::move(pet);
		}

		// One taken out at `index` moves those after it down a slot.
		template <typename T>
		void remove(std::array<T, 3> &slots, size_t index)
		{
			if (index >= slots.size())
				return;

			for (size_t i = index; i + 1 < slots.size(); i++)
				slots[i] = std::move(slots[i + 1]);

			slots.back() = T();
		}

		// What double-clicking a pet does (FUN_00a240a2).
		enum class Summon
		{
			// Ask the server to bring it out, or back if it is out.
			SEND,
			// Three are out: "You have exceeded the maximum number..."
			FULL,
			// Another is out: ask whether the new one leads.
			ASK_BOSS
		};

		// With Follow the Lead (`lead`), a pet that is out goes back, a
		// fourth is refused, and one joining another asks whether it leads.
		// Without, the server decides.
		inline Summon summon(bool lead, const std::array<int64_t, 3> &out, int64_t serial)
		{
			if (!lead)
				return Summon::SEND;

			for (int64_t pet : out)
				if (pet == serial)
					return Summon::SEND;

			if (out[2])
				return Summon::FULL;

			return out[0] ? Summon::ASK_BOSS : Summon::SEND;
		}

		// Follow the Lead: the beginner's, or the Noblesse's or Legend's.
		inline int32_t follow_the_lead(int16_t job)
		{
			switch (job / 1000)
			{
				case 1:
					return 10000018;
				case 2:
					return 20000024;
				default:
					return 8;
			}
		}

		// The animation a pet's move action (stance >> 1) shows
		// (FUN_00703c36): 0 move, 1 stand0, 2 stand1, 3 jump, 4 fly,
		// 5 hungry, 6 rest0, 7 rest1, 8 hang. Its last bit is left.
		inline uint8_t shown(uint8_t stance)
		{
			switch (stance >> 1)
			{
				case 2:
					return 1;
				case 3:
					return 3;
				case 6:
					return 4;
				case 11:
					return 2;
				case 12:
					return 5;
				case 13:
					return 6;
				case 14:
					return 7;
				case 15:
					return 8;
				default:
					return 0;
			}
		}
	}
}
