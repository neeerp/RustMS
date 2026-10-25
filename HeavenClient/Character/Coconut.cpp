#include "Coconut.h"

#include <algorithm>
#include <chrono>
#include <cstdio>

namespace ms
{
	namespace coconut
	{
		namespace
		{
			int32_t entered = 0;
			int16_t nuts = DEFAULT_COUNT;
			std::array<Uniform, 4> worn;
		}

		std::string object_name(int16_t index)
		{
			char name[8];
			std::snprintf(name, sizeof(name), "%03d", index);

			return name;
		}

		HitPacket read_hit(InPacket& recv)
		{
			HitPacket hit;
			hit.index = recv.read_short();
			hit.delay = static_cast<uint16_t>(recv.read_short());
			hit.state = static_cast<uint8_t>(recv.read_byte());

			return hit;
		}

		std::array<uint16_t, 2> read_score(InPacket& recv)
		{
			uint16_t maple = static_cast<uint16_t>(recv.read_short());
			uint16_t story = static_cast<uint16_t>(recv.read_short());

			return { maple, story };
		}

		void Hits::add(const HitPacket& hit, int64_t now, int16_t count)
		{
			int64_t due = now + hit.delay;

			if (hit.index >= 0)
			{
				pending.push_back({ hit.index, hit.state, due });

				return;
			}

			for (int16_t i = 0; i < count; i++)
				pending.push_back({ i, hit.state, due });
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

		void Hits::clear()
		{
			pending.clear();
		}

		bool Box::empty() const
		{
			return left >= right || top >= bottom;
		}

		int16_t first_hit(const std::vector<Box>& nuts, const Box& swing)
		{
			for (size_t i = 0; i < nuts.size(); i++)
			{
				const Box& nut = nuts[i];

				if (nut.empty())
					continue;

				Box both{ std::max(nut.left, swing.left), std::max(nut.top, swing.top),
						  std::min(nut.right, swing.right), std::min(nut.bottom, swing.bottom) };

				if (!both.empty())
					return static_cast<int16_t>(i);
			}

			return -1;
		}

		void enter_field(int32_t fieldtype, int16_t count, const std::array<Uniform, 4>& uniforms)
		{
			entered = fieldtype;
			nuts = count;
			worn = uniforms;
			state() = State();
		}

		bool on_field()
		{
			return entered == FIELD_TYPE;
		}

		int16_t count()
		{
			return nuts;
		}

		std::optional<Uniform> uniform(uint8_t team, bool female)
		{
			if (team == NO_TEAM || !on_field())
				return std::nullopt;

			return worn[(team != 0) * 2 + (female ? 1 : 0)];
		}

		State& state()
		{
			static State current;

			return current;
		}

		int64_t now()
		{
			auto since = std::chrono::steady_clock::now().time_since_epoch();

			return std::chrono::duration_cast<std::chrono::milliseconds>(since).count();
		}

		int64_t clock_end(int64_t now, int32_t seconds)
		{
			int64_t end = now + static_cast<int64_t>(seconds) * 1000;

			return end == 0 ? 1 : end;
		}

		std::string score_text(uint16_t score)
		{
			char text[8];
			std::snprintf(text, sizeof(text), "%03d", score);

			return text;
		}

		std::optional<std::string> time_text(int64_t end, int64_t now)
		{
			if (end == 0)
				return std::nullopt;

			int64_t left = std::max<int64_t>(0, (end - now) / 1000);
			char text[16];
			std::snprintf(text, sizeof(text), "%02d:%02d", static_cast<int32_t>(left / 60),
						  static_cast<int32_t>(left % 60));

			return std::string(text);
		}
	}
}
