#pragma once

#include <algorithm>
#include <cstdint>
#include <utility>
#include <vector>

namespace ms
{
	// A fading notice's place in v83's one line of them (CWvsContext+0x367c,
	// counted by DAT_00bed710), shared by every kind: invitations and quest
	// completions alike. Each sits above the ones made before it, and those
	// after one that goes move down (FUN_0051f93d, FUN_0052459c).
	class FadePlace
	{
	public:
		FadePlace() : ticket(++last())
		{
			line().push_back(ticket);
		}

		FadePlace(FadePlace&& other) noexcept : ticket(std::exchange(other.ticket, 0)) {}

		FadePlace& operator=(FadePlace&& other) noexcept
		{
			if (this != &other)
			{
				leave();
				ticket = std::exchange(other.ticket, 0);
			}

			return *this;
		}

		~FadePlace()
		{
			leave();
		}

		// How many notices made before this one are still there.
		size_t index() const
		{
			const std::vector<uint32_t>& tickets = line();

			return std::find(tickets.begin(), tickets.end(), ticket) - tickets.begin();
		}

	private:
		void leave()
		{
			std::vector<uint32_t>& tickets = line();
			tickets.erase(std::remove(tickets.begin(), tickets.end(), ticket), tickets.end());
		}

		static std::vector<uint32_t>& line()
		{
			static std::vector<uint32_t> tickets;
			return tickets;
		}

		static uint32_t& last()
		{
			static uint32_t ticket = 0;
			return ticket;
		}

		// 0 once moved from.
		uint32_t ticket;
	};

	// The life of one of v83's fading Yes/No notices (FUN_0051fb72): it fades
	// in over a second, stays until its time from being made runs out or a
	// button closes it, then fades out over a second and is gone. A close asked
	// for while it is still fading in waits for that to finish. LOCK_UI holds
	// it back: one made while the UI is locked waits to fade in, and one shown
	// fades out, waits, and comes back with its whole time again from the lock
	// (FUN_0051fb31).
	class FadeNotice
	{
	public:
		static constexpr uint32_t FADE_IN = 1000, FADE_OUT = 1000;
		// How far each notice sits above the one made before it.
		static constexpr int16_t CASCADE = 5;

		explicit FadeNotice(uint32_t lifetime) : lifetime(lifetime), deadline(lifetime) {}

		// How far above the first notice's place this one sits.
		int16_t raised() const
		{
			return static_cast<int16_t>(place.index() * CASCADE);
		}

		// `locked`: whether LOCK_UI is on (ctx+0x30b0).
		void update(uint32_t elapsed, bool locked = false)
		{
			if (phase == Phase::WAITING && !locked)
			{
				phase = Phase::FADING_IN;
				fade_start = age;
			}

			age += elapsed;

			switch (phase)
			{
				case Phase::FADING_IN:
					if (age - fade_start > FADE_IN)
						phase = Phase::SHOWN;
					break;
				case Phase::SHOWN:
					if (locked)
					{
						phase = Phase::WAITING;
						fade_start = age;
						hiding = true;
						deadline = static_cast<uint64_t>(age) + lifetime;
					} else if (closing || age > deadline)
					{
						phase = Phase::FADING_OUT;
						fade_start = age;
					}
					break;
				case Phase::FADING_OUT:
					if (age - fade_start > FADE_OUT)
						phase = Phase::GONE;
					break;
				case Phase::GONE:
					break;
			}
		}

		void close()
		{
			closing = true;
		}

		// A button was pressed or its time ran out; the buttons do nothing more.
		bool closed() const
		{
			return closing || phase == Phase::FADING_OUT || phase == Phase::GONE;
		}

		bool gone() const
		{
			return phase == Phase::GONE;
		}

		// Held back by LOCK_UI, out of sight.
		bool waiting() const
		{
			return phase == Phase::WAITING;
		}

		float opacity() const
		{
			switch (phase)
			{
				case Phase::WAITING:
					return hiding ? std::max(0.0f, 1.0f - static_cast<float>(age - fade_start) / FADE_OUT) : 0.0f;
				case Phase::FADING_IN:
					return std::min(1.0f, static_cast<float>(age - fade_start) / FADE_IN);
				case Phase::SHOWN:
					return 1.0f;
				case Phase::FADING_OUT:
					return std::max(0.0f, 1.0f - static_cast<float>(age - fade_start) / FADE_OUT);
				default:
					return 0.0f;
			}
		}

	private:
		enum class Phase { WAITING, FADING_IN, SHOWN, FADING_OUT, GONE };

		uint32_t lifetime;
		// When it fades out by itself, from being made.
		uint64_t deadline;
		uint32_t age = 0, fade_start = 0;
		Phase phase = Phase::WAITING;
		bool closing = false;
		// Fading out for a lock, from when it was shown.
		bool hiding = false;
		FadePlace place;
	};
}
