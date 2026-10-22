#pragma once

#include <array>
#include <cstddef>
#include <cstdint>
#include <optional>
#include <string>

namespace ms
{
	// v83's five floating lines near the top of the screen (CWvsContext+0x3680, FUN_006ce251), which
	// SCRIPT_PROGRESS_MESSAGE writes to. A line takes the first slot that is free: never used, or shown for
	// 3 s already, though it may still be fading. With all five busy the line is dropped. A line stays for
	// 3 s and then fades out over 3 more.
	class FloatingLines
	{
	public:
		static constexpr size_t SLOTS = 5;
		static constexpr int32_t SHOWN = 3000;
		static constexpr int32_t FADE = 3000;

		// The slot `text` went to, if any was free.
		std::optional<size_t> add(const std::string &text)
		{
			for (size_t i = 0; i < SLOTS; i++)
			{
				Slot &slot = slots[i];

				if (slot.age < 0 || slot.age >= SHOWN)
				{
					slot = { text, 0 };
					return i;
				}
			}

			return std::nullopt;
		}

		void update(int32_t elapsed)
		{
			for (Slot &slot : slots)
				if (slot.age >= 0 && slot.age < SHOWN + FADE)
					slot.age += elapsed;
		}

		float opacity(size_t i) const
		{
			int32_t age = slots[i].age;

			if (age < 0 || age >= SHOWN + FADE)
				return 0.0f;

			if (age <= SHOWN)
				return 1.0f;

			return 1.0f - static_cast<float>(age - SHOWN) / FADE;
		}

		const std::string &text(size_t i) const
		{
			return slots[i].text;
		}

	private:
		struct Slot
		{
			std::string text;
			// Since it was shown, in ms; -1 when never used.
			int32_t age = -1;
		};

		std::array<Slot, SLOTS> slots;
	};
}
