#pragma once

#include <algorithm>
#include <array>
#include <cctype>
#include <cstddef>
#include <cstdint>
#include <map>
#include <optional>
#include <string>
#include <utility>
#include <vector>

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

		// A way a pet can answer: an animation and the lines it may say.
		struct Reaction
		{
			std::string act;
			std::vector<std::string> lines;
		};

		// A command (Item.nx Pet/<id>.img/interact/N): the words that call
		// it at levels `min` to `max`, and the answers when it obeys or not.
		struct Command
		{
			int16_t min;
			int16_t max;
			std::vector<std::string> words;
			std::vector<Reaction> success;
			std::vector<Reaction> fail;
		};

		// The text without the spaces, tabs and line breaks around it.
		inline std::string trim(const std::string &text)
		{
			const char *space = " \t\r\n";
			size_t first = text.find_first_not_of(space);

			if (first == std::string::npos)
				return {};

			return text.substr(first, text.find_last_not_of(space) - first + 1);
		}

		// A command's words, as the PetDialog line "Sleep|Sleepy|Doze" lists
		// them (FUN_0070af01). Each word is trimmed in front only, as the
		// original does.
		inline std::vector<std::string> split_words(const std::string &line)
		{
			std::vector<std::string> words;
			std::string rest = trim(line);
			size_t bar;

			while ((bar = rest.find('|')) != std::string::npos)
			{
				words.push_back(rest.substr(0, bar));
				rest = trim(rest.substr(bar + 1));
			}

			if (!rest.empty())
				words.push_back(rest);

			return words;
		}

		// A command the pet heard, and whether it was called by name.
		struct Heard
		{
			uint8_t command;
			bool named;
		};

		// The command a chat line gives a pet named `name`, of the kind
		// `kind`, at `level` (FUN_00704d5d). A renamed pet can be called by
		// name ("Tom! Sleep"); the words must match exactly, case and all.
		inline std::optional<Heard> hear(const std::string &line, const std::string &name, const std::string &kind,
										 int16_t level, const std::vector<Command> &commands)
		{
			std::string text = trim(line);
			bool named = false;

			if (name != kind && text.size() > name.size() && text.compare(0, name.size(), name) == 0)
			{
				text = trim(text.substr(name.size()));

				while (!text.empty() && (text[0] == '!' || text[0] == ',' || text[0] == '~'))
					text = trim(text.substr(1));

				named = true;
			}

			for (size_t i = 0; i < commands.size() && i <= UINT8_MAX; i++)
			{
				const Command &command = commands[i];

				if (level < command.min || level > command.max)
					continue;

				for (const std::string &word : command.words)
					if (word == text)
						return Heard{ static_cast<uint8_t>(i), named };
			}

			return std::nullopt;
		}

		// The animations every pet has (0x00bec21c), by their action number.
		constexpr std::array<const char *, 9> BASE_ACTIONS = {
			"move", "stand0", "stand1", "jump", "fly", "hungry", "rest0", "rest1", "hang"
		};

		// Children of a pet's image that aren't its own animations
		// (FUN_0070af01, compared ignoring case).
		inline bool reserved(const std::string &child)
		{
			static constexpr std::array<const char *, 13> RESERVED = {
				"autoSpeaking", "autoReact", "info", "monolog", "interact", "food", "stand0", "stand1", "jump",
				"hungry", "rest0", "rest1", "hang"
			};

			for (const char *name : RESERVED)
			{
				size_t length = std::char_traits<char>::length(name);

				if (child.size() != length)
					continue;

				bool same = true;

				for (size_t i = 0; i < length && same; i++)
					same = std::tolower(static_cast<unsigned char>(child[i])) ==
						   std::tolower(static_cast<unsigned char>(name[i]));

				if (same)
					return true;
			}

			return false;
		}

		// The action an animation name stands for (FUN_0070d714): 0 to 8
		// for the base ones, 9 on for the pet's own (`customs`, in order),
		// -1 for one it doesn't have.
		inline int16_t action(const std::string &act, const std::vector<std::string> &customs)
		{
			for (size_t i = 0; i < BASE_ACTIONS.size(); i++)
				if (act == BASE_ACTIONS[i])
					return static_cast<int16_t>(i);

			for (size_t i = 0; i < customs.size(); i++)
				if (act == customs[i])
					return static_cast<int16_t>(BASE_ACTIONS.size() + i);

			return -1;
		}

		// The animation name of an action, empty for none.
		inline std::string action_name(int16_t action, const std::vector<std::string> &customs)
		{
			if (action < 0)
				return {};

			auto index = static_cast<size_t>(action);

			if (index < BASE_ACTIONS.size())
				return BASE_ACTIONS[index];

			index -= BASE_ACTIONS.size();

			return index < customs.size() ? customs[index] : std::string();
		}

		// A pet that has stood still `still` ms, `step` ms more than a tick
		// before, may do something on its own: every 5 s (FUN_0070403d).
		inline bool idle_due(int32_t still, int32_t step)
		{
			return still >= 5000 && still % 5000 < step;
		}

		// Pet food (212xxxx) is fed with PET_FOOD rather than used.
		inline bool is_food(int32_t itemid)
		{
			return itemid / 10000 == 212;
		}

		// Cash pet food (524xxxx) is used with USE_CASH_ITEM; the server
		// picks the pet it suits.
		inline bool is_cash_food(int32_t itemid)
		{
			return itemid / 10000 == 524;
		}

		// The Pet Name Tag (5170000) names the first pet out.
		inline bool is_name_tag(int32_t itemid)
		{
			return itemid / 10000 == 517 && itemid % 10000 == 0;
		}

		// A pet's name is 4 to 12 bytes (FUN_009ac7cb).
		constexpr size_t MIN_NAME = 4;
		constexpr size_t MAX_NAME = 12;

		enum class Naming
		{
			OK,
			// Under 4 bytes: the prompt asks again (FUN_009ab50a).
			SHORT,
			// A control character other than a line break.
			CURSING,
			// The kind's own name, in any case, or the pet's.
			TAKEN
		};

		// Whether two names match, ignoring case (FUN_005392d4).
		inline bool same_name(const std::string &a, const std::string &b)
		{
			if (a.size() != b.size())
				return false;

			for (size_t i = 0; i < a.size(); i++)
				if (std::toupper(static_cast<unsigned char>(a[i])) != std::toupper(static_cast<unsigned char>(b[i])))
					return false;

			return true;
		}

		// Check the name the player typed for a pet of kind `species` now
		// called `current` (FUN_00a0a63f type 0x11). `typed` becomes the
		// name to send: trimmed, with curse words starred out by `censor`
		// and tabs as spaces.
		template <typename Censor>
		Naming check_name(std::string &typed, const std::string &species, const std::string &current, Censor censor)
		{
			typed = trim(typed);

			if (typed.size() < MIN_NAME)
				return Naming::SHORT;

			for (char c : typed)
				if (static_cast<unsigned char>(c) < 0x20 && c != '\r' && c != '\n')
					return Naming::CURSING;

			typed = censor(typed);
			std::replace(typed.begin(), typed.end(), '\t', ' ');

			if (same_name(typed, species) || typed == current)
				return Naming::TAKEN;

			return Naming::OK;
		}

		// The pet out that eats: the one least full, the first of equals
		// (FUN_00a09905). -1 marks a slot without a pet; none gives -1.
		inline int hungriest(const std::array<int16_t, 3> &fullness)
		{
			int index = -1;

			for (int i = 0; i < static_cast<int>(fullness.size()); i++)
				if (fullness[i] >= 0 && (index < 0 || fullness[i] < fullness[index]))
					index = i;

			return index;
		}

		// The food answers (`food/N`) for a pet at `level`: the first entry
		// whose levels hold it (FUN_007048ab).
		inline const Command *food_for(const std::vector<Command> &food, int16_t level)
		{
			for (const Command &entry : food)
				if (level >= entry.min && level <= entry.max)
					return &entry;

			return nullptr;
		}

		// The status line for a pet whose closeness went from `before` to
		// `now`, empty if it didn't change (FUN_00704c18).
		inline std::string closeness_change(int32_t before, int32_t now)
		{
			if (now > before)
				return "The closeness increased (+" + std::to_string(now - before) + ")";

			if (now < before)
				return "The closeness decreased (" + std::to_string(now - before) + ")";

			return {};
		}

		// Pet equipment goes on body parts 14 and 21 to 48, which the
		// three pets in the list share out between them.
		inline bool is_equip_part(int16_t part)
		{
			return part == 14 || (part >= 21 && part <= 48);
		}

		// The parts pet equipment `itemid` goes on for each pet in the list
		// (FUN_00460358), the same for all three for the HP and MP pouches.
		// None for an item that isn't pet equipment.
		// The ability items that change what a pet picks up (FUN_00705e2c).
		constexpr int32_t MESO_MAGNET = 1812000;
		constexpr int32_t ITEM_POUCH = 1812001;
		constexpr int32_t SWEEPER = 1812004;
		constexpr int32_t LONG_RANGE = 1812005;
		constexpr int32_t MAGIC_SCALES = 1812006;

		inline std::optional<std::array<int16_t, 3>> equip_parts(int32_t itemid)
		{
			switch (itemid)
			{
				case 1802100:
					return std::array<int16_t, 3>{ 21, 31, 39 };
				case 1812000:
					return std::array<int16_t, 3>{ 23, 34, 42 };
				case 1812001:
					return std::array<int16_t, 3>{ 22, 33, 41 };
				case 1812002:
					return std::array<int16_t, 3>{ 24, 24, 24 };
				case 1812003:
					return std::array<int16_t, 3>{ 25, 25, 25 };
				case 1812004:
					return std::array<int16_t, 3>{ 26, 35, 43 };
				case 1812005:
					return std::array<int16_t, 3>{ 27, 36, 44 };
				case 1812006:
					return std::array<int16_t, 3>{ 28, 37, 45 };
				case 1812007:
					return std::array<int16_t, 3>{ 46, 47, 48 };
				default:
					break;
			}

			switch (itemid / 10000)
			{
				case 180:
					return std::array<int16_t, 3>{ 14, 30, 38 };
				case 182:
					return std::array<int16_t, 3>{ 21, 31, 39 };
				case 183:
					return std::array<int16_t, 3>{ 29, 32, 40 };
				default:
					return std::nullopt;
			}
		}

		// The pet in the list wearing what is on `part`, -1 for the shared
		// pouches (FUN_00705e2c).
		inline int8_t equip_owner(int16_t part)
		{
			if (part == 14 || (part >= 21 && part <= 29 && part != 24 && part != 25) || part == 46)
				return 0;

			if ((part >= 30 && part <= 37) || part == 47)
				return 1;

			if ((part >= 38 && part <= 45) || part == 48)
				return 2;

			return -1;
		}

		// The pet whose clothes are on `part` (14, 30 and 38), or -1.
		inline int8_t wear_owner(int16_t part)
		{
			switch (part)
			{
				case 14:
					return 0;
				case 30:
					return 1;
				case 38:
					return 2;
				default:
					return -1;
			}
		}

		// A pet in the list, as far as putting equipment on it goes.
		struct Wearer
		{
			bool out = false;
			// The item is clothing made for its kind
			// (Character/PetEquip/<item>.img/<kind>).
			bool suits = false;
			// Its kind shows a label ring's name tag, and a quote ring's
			// balloon (Item.nx Pet/<kind>.img/info/nameTag, chatBalloon).
			bool name_tag = false;
			bool chat_balloon = false;
			int16_t level = 0;
		};

		// StringPool 0x36f, 0x370 and 0x364.
		constexpr const char *NO_PET = "You do not currently have a pet that is alive.";
		constexpr const char *CANNOT_WEAR = "The pet is unable to wear the equipment.";
		constexpr const char *STATS_TOO_LOW = "Your stats are too low to equip this item.";

		// Why equipment needing level `req_level` can't go on `part` of
		// `pet`, or null if it can (FUN_004f1c2d). Clothes need a pet of
		// the kind they were made for, at the level, and the rings a kind
		// that shows them. The potion pouches, the Wing Boots, the Binocular
		// and the Magic Scales need no pet at all.
		inline const char *refuse_equip(int16_t part, int16_t req_level, const Wearer &pet)
		{
			switch (part)
			{
				case 14:
				case 30:
				case 38:
					if (!pet.out || !pet.suits)
						return CANNOT_WEAR;

					// FUN_004f2cee compares the pet's level, not the owner's.
					if (pet.level < req_level)
						return STATS_TOO_LOW;

					return nullptr;
				case 21:
				case 31:
				case 39:
					if (!pet.out)
						return NO_PET;

					return pet.name_tag ? nullptr : CANNOT_WEAR;
				case 29:
				case 32:
				case 40:
					if (!pet.out)
						return NO_PET;

					return pet.chat_balloon ? nullptr : CANNOT_WEAR;
				case 22:
				case 23:
				case 46:
				case 33:
				case 34:
				case 47:
				case 41:
				case 42:
				case 48:
					return pet.out ? nullptr : NO_PET;
				default:
					return nullptr;
			}
		}

		// A pet that pet equipment can go on: its place in the list, and
		// the part.
		struct EquipChoice
		{
			uint8_t pet;
			int16_t part;

			bool operator==(const EquipChoice &other) const
			{
				return pet == other.pet && part == other.part;
			}
		};

		// The pets a double-click in the item window can put pet equipment
		// `itemid` on (FUN_004efd25), given the three places in the list.
		// With one pet out or none, and for the potion pouches, it is the
		// first pet's part whether it can wear it or not. Otherwise clothes
		// go on the pets of their kind, ability items on any, and only the
		// Pet Label Ring and the Pet Quote Ring of the rings may choose.
		inline std::vector<EquipChoice> equip_choices(int32_t itemid, const std::array<Wearer, 3> &pets)
		{
			std::optional<std::array<int16_t, 3>> parts = equip_parts(itemid);

			if (!parts)
				return {};

			int16_t first = (*parts)[0];

			if (first == 24 || first == 25 || !pets[1].out)
				return { { 0, first } };

			std::vector<EquipChoice> choices;

			for (uint8_t i = 0; i < pets.size(); i++)
			{
				bool fits = itemid / 10000 == 180 ? pets[i].suits
							: itemid / 10000 == 181 || itemid == 1822000 || itemid == 1832000;

				if (pets[i].out && fits)
					choices.push_back({ i, (*parts)[i] });
			}

			return choices;
		}

		// What the Character Info window's pet page lists for the pet at
		// `index` (FUN_00904aea): the clothes CHAR_INFO says it wears, then
		// the ability items (181xxxx), Pet Label Ring and Pet Quote Ring its
		// owner's look wears for it, by part. The potion pouches serve every
		// pet; of the rest, parts 31 to 37 and 47 are the second's, 39 to 45
		// and 48 the third's, and any other the first's.
		inline std::vector<int32_t> info_items(size_t index, int32_t wear, const std::map<int16_t, int32_t> &parts)
		{
			std::vector<int32_t> items;

			if (wear != 0)
				items.push_back(wear);

			for (const auto &[part, itemid] : parts)
			{
				if (part < 0 || part >= 52 || itemid <= 0)
					continue;

				if (itemid / 10000 != 181 && itemid != 1822000 && itemid != 1832000)
					continue;

				size_t owner = (part >= 31 && part <= 37) || part == 47 ? 1
							   : (part >= 39 && part <= 45) || part == 48 ? 2
							   : 0;

				if (itemid == 1812002 || itemid == 1812003 || owner == index)
					items.push_back(itemid);
			}

			return items;
		}
	}
}
