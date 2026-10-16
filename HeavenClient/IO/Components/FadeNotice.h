#pragma once

#include <algorithm>
#include <cstdint>

namespace ms
{
	// The life of one of v83's fading Yes/No notices (FUN_0051fb72): it fades
	// in over a second, stays until its time from being made runs out or a
	// button closes it, then fades out over a second and is gone. A close asked
	// for while it is still fading in waits for that to finish.
	class FadeNotice
	{
	public:
		static constexpr uint32_t FADE_IN = 1000, FADE_OUT = 1000;

		explicit FadeNotice(uint32_t lifetime) : lifetime(lifetime) {}

		void update(uint32_t elapsed)
		{
			age += elapsed;

			switch (phase)
			{
				case Phase::FADING_IN:
					if (age > FADE_IN)
						phase = Phase::SHOWN;
					break;
				case Phase::SHOWN:
					if (closing || age > lifetime)
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

		float opacity() const
		{
			switch (phase)
			{
				case Phase::FADING_IN:
					return std::min(1.0f, static_cast<float>(age) / FADE_IN);
				case Phase::SHOWN:
					return 1.0f;
				case Phase::FADING_OUT:
					return std::max(0.0f, 1.0f - static_cast<float>(age - fade_start) / FADE_OUT);
				default:
					return 0.0f;
			}
		}

	private:
		enum class Phase { FADING_IN, SHOWN, FADING_OUT, GONE };

		uint32_t lifetime;
		uint32_t age = 0, fade_start = 0;
		Phase phase = Phase::FADING_IN;
		bool closing = false;
	};
}
