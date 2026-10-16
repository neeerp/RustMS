#pragma once

#include <algorithm>
#include <cstdint>
#include <vector>

namespace ms
{
	// How one of v83's buff icons counts down (FUN_007b4819, FUN_007b44f4).
	// Its shadow is one of 16 Skill/CoolTime frames, stepping every
	// sixteenth of the duration it started with. Once 3 s or less are left it
	// blinks: icon and shadow fade from alpha 0xd2 to 0x40 over 500 ms and
	// start over. The icon stays until the server cancels the buff.
	class BuffTimer
	{
	public:
		static constexpr int32_t SHADOWS = 16;
		static constexpr int32_t BLINK_TIME = 3000, BLINK_PERIOD = 500;
		static constexpr int32_t ALPHA = 0xd2, BLINK_ALPHA = 0x40;

		explicit BuffTimer(int32_t duration) : unit(duration / SHADOWS), left(duration) {}

		void update(int32_t elapsed)
		{
			int32_t before = left;
			left -= elapsed;

			if (blinking)
				blink += elapsed;
			else if (before > BLINK_TIME && left <= BLINK_TIME)
				blinking = true;
		}

		// The CoolTime frame: 15 is clear, 0 is shaded.
		int32_t shadow() const
		{
			if (unit <= 0)
				return 0;

			return std::clamp(left / unit, 0, SHADOWS - 1);
		}

		float opacity() const
		{
			if (!blinking)
				return ALPHA / 255.0f;

			float step = static_cast<float>(blink % BLINK_PERIOD) / BLINK_PERIOD;

			return (ALPHA + (BLINK_ALPHA - ALPHA) * step) / 255.0f;
		}

	private:
		int32_t unit, left;
		int32_t blink = 0;
		bool blinking = false;
	};

	// v83's buff row (FUN_007b24d5, FUN_007b2717): one icon per buff, the
	// newest on the right. A buff on stats another icon shows takes them over,
	// so a recast replaces its old icon. An icon goes when the server has
	// cancelled every stat it shows.
	template <typename Icon>
	class BuffRow
	{
	public:
		struct Entry
		{
			int32_t id;
			std::vector<int32_t> stats;
			BuffTimer timer;
			Icon icon;
		};

		// GIVE_BUFF names a buff once for each of its stats. `make` builds the
		// icon when a new one is needed.
		template <typename Make>
		void add(int32_t id, int32_t stat, int32_t duration, Make make)
		{
			auto same = find(id);

			if (same != entries.end() && !shows(*same, stat))
			{
				same->stats.push_back(stat);
				drop(stat, &id);

				return;
			}

			if (same != entries.end())
				entries.erase(same);

			drop(stat, nullptr);
			entries.push_back({ id, { stat }, BuffTimer(duration), make() });
		}

		void cancel(int32_t stat)
		{
			drop(stat, nullptr);
		}

		void update(int32_t elapsed)
		{
			for (auto& entry : entries)
				entry.timer.update(elapsed);
		}

		// Left to right.
		const std::vector<Entry>& icons() const
		{
			return entries;
		}

	private:
		typename std::vector<Entry>::iterator find(int32_t id)
		{
			return std::find_if(entries.begin(), entries.end(), [id](const Entry& entry) { return entry.id == id; });
		}

		static bool shows(const Entry& entry, int32_t stat)
		{
			return std::find(entry.stats.begin(), entry.stats.end(), stat) != entry.stats.end();
		}

		// Takes the stat from every icon but the one for `keep`.
		void drop(int32_t stat, const int32_t* keep)
		{
			for (auto iter = entries.begin(); iter != entries.end();)
			{
				if (!keep || iter->id != *keep)
					iter->stats.erase(std::remove(iter->stats.begin(), iter->stats.end(), stat), iter->stats.end());

				if (iter->stats.empty())
					iter = entries.erase(iter);
				else
					iter++;
			}
		}

		std::vector<Entry> entries;
	};
}
