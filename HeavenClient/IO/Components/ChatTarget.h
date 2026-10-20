#pragma once

#include <algorithm>
#include <cstddef>
#include <cstdint>
#include <deque>
#include <string>
#include <vector>

namespace ms
{
	namespace chat_target
	{
		// Who a typed line goes to. The order is that of v83's chat target
		// list (FUN_008dfb36), whose item index is the mode.
		enum class Target : uint8_t
		{
			BUDDY,
			GROUP,
			PARTY,
			GUILD,
			ALLIANCE,
			SPOUSE,
			WHISPER,
			ALL
		};

		constexpr size_t COUNT = 8;

		// The list's items (StringPool 0x312-0x317 and 0x15dd).
		inline const char* item_text(Target target)
		{
			switch (target)
			{
				case Target::BUDDY:
					return "To a buddy";
				case Target::GROUP:
					return "To Group";
				case Target::PARTY:
					return "To the party";
				case Target::GUILD:
					return "To the Guild";
				case Target::ALLIANCE:
					return "To Alliance";
				case Target::SPOUSE:
					return "To Spouse";
				case Target::WHISPER:
					return "Whisper";
				default:
					return "To All";
			}
		}

		// The box shows the mode rather than the item picked: the buddy group
		// in brackets, cut to 8 characters and "..." past 12, and whoever is
		// whispered to (FUN_004c5fb9).
		inline std::string face_text(Target target, const std::string& group, const std::string& whisper)
		{
			if (target == Target::GROUP && !group.empty())
				return "[" + (group.size() > 12 ? group.substr(0, 8) + "..." : group) + "]";

			if (target == Target::WHISPER && !whisper.empty())
				return whisper;

			return item_text(target);
		}

		// The target Tab moves on to (FUN_008d536c): all, buddy, spouse, group,
		// party, guild, alliance, whisper and back to all.
		inline Target next_tab(Target target)
		{
			switch (target)
			{
				case Target::ALL:
					return Target::BUDDY;
				case Target::BUDDY:
					return Target::SPOUSE;
				case Target::SPOUSE:
					return Target::GROUP;
				case Target::GROUP:
					return Target::PARTY;
				case Target::PARTY:
					return Target::GUILD;
				case Target::GUILD:
					return Target::ALLIANCE;
				case Target::ALLIANCE:
					return Target::WHISPER;
				default:
					return Target::ALL;
			}
		}

		// What the "To Group" dialog offers (FUN_00a3fc85): a blank entry,
		// then each buddy's group once, in bytewise order. Pending buddies'
		// groups count too.
		inline std::vector<std::string> group_choices(std::vector<std::string> groups)
		{
			std::sort(groups.begin(), groups.end());

			std::vector<std::string> choices = { "" };

			for (const std::string& group : groups)
				if (!group.empty() && group != choices.back())
					choices.push_back(group);

			return choices;
		}

		// The names last whispered to, the latest first, each once and at
		// most ten (FUN_008e0a37). Names are compared exactly.
		class WhisperHistory
		{
		public:
			static constexpr size_t MAX = 10;

			void add(const std::string& name)
			{
				for (auto it = names.begin(); it != names.end(); ++it)
				{
					if (*it == name)
					{
						names.erase(it);
						break;
					}
				}

				names.push_front(name);

				if (names.size() > MAX)
					names.pop_back();
			}

			// The latest name, or nothing.
			std::string latest() const
			{
				return names.empty() ? std::string() : names.front();
			}

			const std::deque<std::string>& all() const
			{
				return names;
			}

			// The names left in the whisper prompt's list when OK is pressed
			// (FUN_009acea9).
			void replace(const std::vector<std::string>& kept)
			{
				names.assign(kept.begin(), kept.begin() + std::min(kept.size(), MAX));
			}

		private:
			std::deque<std::string> names;
		};
	}
}
