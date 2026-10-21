#include "QuestRequirements.h"

#include "Job.h"

#include <array>

namespace ms::QuestRequirements
{
	namespace
	{
		// v83 spells "Avaliable" so (StringPool 0xcaa).
		const std::string ALL_EXCEPT = "Avaliable to all except ";

		// The advancement form (FUN_00888304, FUN_00888414, FUN_00888474, FUN_008884f0): 12 slots, the five
		// explorer classes then the Cygnus ones, each with 3 tiers (2nd to 4th job) of 3, 2 or 1 branches.
		// It fails, and the caller lists names instead, unless every id is a 2nd to 4th job one, every tier
		// has all its branches or none, and every slot used covers the same tiers.
		bool advancement(const std::vector<int32_t>& ids, std::string& out)
		{
			constexpr size_t SLOTS = 12;
			constexpr size_t TIERS = 3;
			constexpr std::array<int32_t, SLOTS> BRANCHES = { 3, 3, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1 };

			std::array<std::array<std::array<bool, 3>, TIERS>, SLOTS> cells = {};

			for (int32_t id : ids)
			{
				// Aran and Evan ids land in the Warrior and Magician slots, as in v83; first jobs and
				// beginners fail.
				int32_t slot = (id / 1000 == 1 ? 4 : -1) + (id / 100) % 10;
				int32_t tier = id % 10;
				int32_t branch = (id / 10) % 10 - 1;

				if (id < 0 || slot < 0 || slot >= static_cast<int32_t>(SLOTS) || tier > 2 || branch < 0
					|| branch >= BRANCHES[slot])
					return false;

				cells[slot][tier][branch] = true;
			}

			auto count = [&](size_t slot, size_t tier)
			{
				int32_t n = 0;

				for (int32_t branch = 0; branch < BRANCHES[slot]; branch++)
					n += cells[slot][tier][branch];

				return n;
			};

			bool any_full = false;

			for (size_t slot = 0; slot < SLOTS; slot++)
			{
				for (size_t tier = 0; tier < TIERS; tier++)
				{
					int32_t n = count(slot, tier);

					if (n != 0 && n != BRANCHES[slot])
						return false;

					any_full = any_full || n == BRANCHES[slot];
				}
			}

			if (!any_full)
				return false;

			int32_t first_all = -1;
			int32_t last_all = -1;
			std::array<bool, SLOTS> used = {};

			for (size_t slot = 0; slot < SLOTS; slot++)
			{
				int32_t first = -1;
				int32_t last = -1;

				for (size_t tier = 0; tier < TIERS; tier++)
				{
					bool full = count(slot, tier) == BRANCHES[slot];

					if (first < 0)
					{
						if (full)
							first = static_cast<int32_t>(tier);
					}
					else if (last < 0)
					{
						if (!full)
							last = static_cast<int32_t>(tier) - 1;
					}
					else if (full)
					{
						// A gap between tiers.
						return false;
					}
				}

				if (first < 0)
					continue;

				if (last < 0)
					last = 2;

				if (first_all < 0)
				{
					first_all = first;
					last_all = last;
				}
				else if (first != first_all || last != last_all)
				{
					return false;
				}

				used[slot] = true;
			}

			// Only the first tier is told: a range is "at least" it, even one ending before the 4th job.
			static const std::array<std::string, TIERS> ORDINALS = { "2nd", "3rd", "4th" };
			static const std::array<std::string, 5> CLASSES = { "Warrior", "Magician", "Bowman", "Thief", "Pirate" };

			out = (first_all == last_all ? "" : "At least at ") + ORDINALS[first_all] + " job advancement ";

			std::string list;
			std::string excluded;
			size_t n = 0;

			for (size_t slot = 0; slot < SLOTS; slot++)
			{
				// The Cygnus slots go by their first job's name; v83 names no 16xx or 17xx.
				std::string name = slot < CLASSES.size() ? CLASSES[slot]
														 : Job::get_name(static_cast<uint16_t>(600 + 100 * slot));

				if (used[slot])
				{
					list += (list.empty() ? "" : ", ") + name;
					n++;
				}
				else if (excluded.empty())
				{
					excluded = name;
				}
			}

			// No real ids reach the last two slots, so these two never show.
			if (n == SLOTS)
				out += "Available to all";
			else if (n == SLOTS - 1)
				out += ALL_EXCEPT + excluded;
			else
				out += "Available to " + list;

			return true;
		}
	}

	std::string levels(bool checked, int32_t lvmin, int32_t lvmax)
	{
		if (!checked)
			return "No limit";

		std::string text;

		if (lvmin != 0)
			text = "Over Level " + std::to_string(lvmin);

		if (lvmax != 0)
			text = (text.empty() ? "" : text + " ") + " Under Level " + std::to_string(lvmax);

		return text;
	}

	std::string jobs(const std::vector<int32_t>& ids)
	{
		if (ids.empty())
			return "Available to all";

		std::vector<int32_t> warrior, magician, bowman, thief, pirate, aran, evan;
		std::vector<int32_t> dawn, blaze, wind, night, thunder, cygnus;
		bool beginner = false, noblesse = false, legend = false, evan0 = false;

		for (int32_t id : ids)
		{
			switch (id / 100)
			{
				case 1: warrior.push_back(id); break;
				case 2: magician.push_back(id); break;
				case 3: bowman.push_back(id); break;
				case 4: thief.push_back(id); break;
				case 5: pirate.push_back(id); break;
				case 11: dawn.push_back(id); break;
				case 12: blaze.push_back(id); break;
				case 13: wind.push_back(id); break;
				case 14: night.push_back(id); break;
				case 15: thunder.push_back(id); break;
				case 21: aran.push_back(id); break;
				case 22: evan.push_back(id); break;
				default: break;
			}

			if (id / 1000 == 1)
				cygnus.push_back(id);

			beginner = beginner || id == 0;
			noblesse = noblesse || id == 1000;
			legend = legend || id == 2000;
			evan0 = evan0 || id == 2001;
		}

		// "Everyone but" lists, told apart only by their length.
		size_t n = ids.size();

		if (n == 75 && !beginner && !noblesse && !legend && !evan0)
			return "Not available for Beginners, Noblesses, and Legends.";

		if (n == 58 && dawn.empty() && blaze.empty() && wind.empty() && night.empty() && thunder.empty())
			return ALL_EXCEPT + "Cygnus Knights";

		if (n == 74 && aran.empty())
			return ALL_EXCEPT + "Aran";

		if (n == 37 && warrior.empty() && magician.empty() && bowman.empty() && thief.empty() && pirate.empty())
			return ALL_EXCEPT + "Adventurer";

		if (n == 68 && evan.empty())
			return ALL_EXCEPT + "Evan";

		std::string text;

		if (advancement(ids, text))
			return text;

		auto add = [&](const std::string& name)
		{
			text += (text.empty() ? "" : ", ") + name;
		};

		// A whole class by its name, otherwise each job in it.
		auto add_class = [&](const std::vector<int32_t>& jobs, size_t whole, const std::string& name)
		{
			if (jobs.size() == whole)
				add(name);
			else
				for (int32_t id : jobs)
					add(Job::get_name(static_cast<uint16_t>(id)));
		};

		if (beginner)
			add("Beginner");

		if (noblesse && cygnus.size() != 21)
			add("Noblesse");

		if (legend && aran.size() != 4)
			add("Legend");

		if (evan0 && evan.size() != 10)
			add("Evan0");

		add_class(warrior, 10, "Warrior");
		add_class(magician, 10, "Magician");
		add_class(bowman, 7, "Bowman");
		add_class(thief, 7, "Thief");
		add_class(pirate, 7, "Pirate");
		add_class(aran, 4, "Aran");
		add_class(evan, 10, "Evan");

		if (cygnus.size() == 21)
		{
			add("Cygnus Knights");
		}
		else
		{
			add_class(dawn, 4, "Dawn Warrior");
			add_class(blaze, 4, "Blaze Wizard");
			add_class(wind, 4, "Wind Archer");
			add_class(night, 4, "Night Walker");
			add_class(thunder, 4, "Thunder Breaker");
		}

		return "Available to " + text;
	}
}
