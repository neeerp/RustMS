#pragma once

#include <array>
#include <cstdint>
#include <vector>

namespace ms
{
	// The sets of Effect/SetEff.img: a character who wears one of each of a set's groups of items shows its
	// effect around them (v83 FUN_005d925d, from FUN_009393fc).
	namespace set_effect
	{
		// A set marked `cash` wins over the others, and one with a `multipet` key loses to them.
		enum class Kind : uint8_t
		{
			PLAIN,
			CASH,
			MULTIPET
		};

		// One of `info`'s children: its name, and the items of which one must be worn.
		struct Group
		{
			int32_t key;
			std::vector<int32_t> items;
		};

		struct Set
		{
			Kind kind = Kind::PLAIN;
			std::vector<Group> groups;
			// `info/multipet`: every one of the three pets must be one of these.
			std::vector<int32_t> pets;
		};

		// What a character wears, by v83 body part: 0 is the hair, 11 the weapon, 12, 13, 15 and 16 the rings.
		// It is the look's, so a cash item stands for the regular one under it.
		using Parts = std::array<int32_t, 51>;

		// The body parts an item is worn in (FUN_004606a0): a ring in any of the four ring parts, a weapon or a
		// cash weapon in the weapon's, and none for an item that isn't worn.
		inline std::vector<int16_t> parts_of(int32_t item_id)
		{
			switch (item_id / 10000)
			{
				case 100:
					return { 1 };
				case 101:
					return { 2 };
				case 102:
					return { 3 };
				case 103:
					return { 4 };
				case 104:
				case 105:
					return { 5 };
				case 106:
					return { 6 };
				case 107:
					return { 7 };
				case 108:
					return { 8 };
				case 109:
					return { 10 };
				case 110:
					return { 9 };
				case 111:
					return { 12, 13, 15, 16 };
				case 112:
					return { 17 };
				case 113:
					return { 50 };
				case 114:
					return { 49 };
				case 170:
					return { 11 };
				default:
					if (item_id / 10000 >= 130 && item_id / 10000 < 150)
						return { 11 };

					return {};
			}
		}

		// Whether one of a group's items is worn. Group 0 is the hair; any other group looks in the parts of its
		// first item. A cash weapon stands in for the weapon under it.
		inline bool satisfied(const Group &group, const Parts &worn, int32_t cash_weapon)
		{
			if (group.items.empty())
				return true;

			std::vector<int16_t> parts = group.key == 0 ? std::vector<int16_t>{ 0 } : parts_of(group.items.front());

			for (int16_t part : parts)
			{
				int32_t item = part == 11 && cash_weapon != 0 ? cash_weapon : worn[part];

				for (int32_t id : group.items)
					if (id == item)
						return true;
			}

			return false;
		}

		// Whether all three pets are out and each is one of the set's.
		inline bool pets_match(const std::vector<int32_t> &list, const std::array<int32_t, 3> &pets)
		{
			if (list.empty())
				return true;

			for (int32_t pet : pets)
			{
				bool listed = false;

				for (int32_t id : list)
					if (id == pet)
						listed = true;

				if (pet == 0 || !listed)
					return false;
			}

			return true;
		}

		inline bool matches(const Set &set, const Parts &worn, int32_t cash_weapon, const std::array<int32_t, 3> &pets)
		{
			for (const Group &group : set.groups)
				if (!satisfied(group, worn, cash_weapon))
					return false;

			return pets_match(set.pets, pets);
		}

		// The set a character shows, or -1: the first `cash` set they complete, else the first plain one, else
		// the first `multipet` one.
		inline int32_t match(const std::vector<Set> &sets, const Parts &worn, int32_t cash_weapon,
							 const std::array<int32_t, 3> &pets)
		{
			int32_t plain = -1;
			int32_t multipet = -1;

			for (size_t i = 0; i < sets.size(); i++)
			{
				if (!matches(sets[i], worn, cash_weapon, pets))
					continue;

				auto index = static_cast<int32_t>(i);

				if (sets[i].kind == Kind::CASH)
					return index;

				// A second pets' set takes the plain one's place, if that is free (asm 5d93a4).
				if (sets[i].kind == Kind::MULTIPET && multipet < 0)
					multipet = index;
				else if (plain < 0)
					plain = index;
			}

			return plain >= 0 ? plain : multipet;
		}

		// The background shows while the character stands. It fades out over half a second once they don't, and
		// comes back, from its first frame, after they have stood for more than a second and a half
		// (FUN_00939a28, every frame from FUN_00930b27).
		constexpr int64_t FADE = 500;
		constexpr int64_t RETURN = 1500;

		struct Background
		{
			bool shown = true;
			int64_t hidden_at = 0;
		};

		// Steps the background at `now`, and returns whether it comes back.
		inline bool step(Background &background, bool standing, int64_t now)
		{
			// Every moment they don't stand counts from again.
			if (!standing)
			{
				background.shown = false;
				background.hidden_at = now;
			} else if (!background.shown && now - background.hidden_at > RETURN)
			{
				background.shown = true;

				return true;
			}

			return false;
		}

		// The background's opacity after a step of `ms` towards shown or hidden.
		inline float fade(float opacity, bool shown, int64_t ms)
		{
			float delta = static_cast<float>(ms) / FADE;

			if (shown)
				return opacity + delta < 1.0f ? opacity + delta : 1.0f;

			return opacity - delta > 0.0f ? opacity - delta : 0.0f;
		}
	}
}
