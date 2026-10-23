#pragma once

#include "QuestInfo.h"

#include <algorithm>
#include <cstdint>
#include <cstdlib>
#include <string>
#include <utility>
#include <vector>

namespace ms
{
	// Values match UIWindow2.img/QuestIcon animation IDs.
	enum class QuestMarker : int8_t { NONE = -1, AVAILABLE = 0, IN_PROGRESS = 1, READY = 2 };

	// Kill counts in a started quest's progress string: three digits per
	// required mob ("003012" for 3 and 12).
	inline std::vector<int16_t> decode_kills(const std::string& progress)
	{
		std::vector<int16_t> kills;
		for (size_t i = 0; i + 3 <= progress.size(); i += 3)
		{
			int16_t count = 0;
			for (size_t j = i; j < i + 3; j++)
				count = count * 10 + (progress[j] >= '0' && progress[j] <= '9' ? progress[j] - '0' : 0);
			kills.push_back(count);
		}
		return kills;
	}

	// Whether a started quest's progress string covers its required kills.
	inline bool end_mobs_met(const QuestInfo& quest, const std::string& progress)
	{
		std::vector<int16_t> kills = decode_kills(progress);
		for (size_t i = 0; i < quest.end_mobs.size(); i++)
			if ((i < kills.size() ? kills[i] : 0) < quest.end_mobs[i].second)
				return false;
		return true;
	}

	// Whether a quest's Check/1/infoex holds for `record`, the data string
	// of its infoNumber quest, or nullptr if that quest isn't started
	// (FUN_00721d2c). One entry must hold: cond 0 compares the strings, 1
	// needs at least the value and 2 at most, as numbers. Entries without
	// an infoNumber (Meren's 6029) are left to the server, which keeps no
	// such string.
	inline bool end_info_met(const QuestInfo& quest, const std::string* record)
	{
		if (quest.end_info_record == 0 || quest.end_info.empty())
			return true;

		if (record == nullptr)
			return false;

		int32_t number = std::atoi(record->c_str());

		return std::any_of(quest.end_info.begin(), quest.end_info.end(), [&](const auto& entry)
		{
			int32_t value = std::atoi(entry.second.c_str());

			switch (entry.first)
			{
				case 1: return value <= number;
				case 2: return number <= value;
				default: return *record == entry.second;
			}
		});
	}

	// Whether the player's items satisfy a quest's completion requirements.
	// `count` gives how many of an item the player holds.
	template<class Count>
	bool end_items_met(const QuestInfo& quest, Count count)
	{
		return std::all_of(quest.end_items.begin(), quest.end_items.end(), [&](const auto& requirement)
		{
			int32_t held = count(requirement.first);
			return requirement.second > 0 ? held >= requirement.second : held == 0;
		});
	}

	// Why a started quest can't be completed with an NPC, by v83's first
	// failed check (FUN_00721d2c), or NONE. The names are the stop/<reason>
	// pages they show.
	// MESO has no stop key of its own, so it shows stop/default. NO_PET and
	// WRONG_PET (codes 8 and 9) show a notice instead of a page.
	enum class QuestStop { NONE, NPC, ITEM, QUEST, INFO, MOB, MESO, NO_PET, WRONG_PET };

	// Whether the first pet out, `first_pet` (0 for none), meets a quest's
	// pet requirement: v83 looks at no other pet (FUN_00721163,
	// FUN_00721d2c).
	inline QuestStop pet_stop(const std::vector<int32_t>& pets, int32_t first_pet)
	{
		if (pets.empty())
			return QuestStop::NONE;

		if (first_pet == 0)
			return QuestStop::NO_PET;

		return std::find(pets.begin(), pets.end(), first_pet) == pets.end() ? QuestStop::WRONG_PET : QuestStop::NONE;
	}

	inline const char* stop_reason(QuestStop stop)
	{
		switch (stop)
		{
			case QuestStop::NPC: return "npc";
			case QuestStop::ITEM: return "item";
			case QuestStop::QUEST: return "quest";
			case QuestStop::INFO: return "info";
			case QuestStop::MOB: return "mob";
			default: return "default";
		}
	}

	// `progress` is the quest's progress string (its kill counts),
	// `first_pet` the item of the first pet out, and `info_record` the data
	// string of the quest's infoNumber quest (see end_info_met).
	template<class Status, class Count>
	QuestStop completion_stop(const QuestInfo& quest, int32_t npc_id, Status status, Count count,
		const std::string& progress, int64_t meso, int32_t first_pet = 0,
		const std::string* info_record = nullptr)
	{
		if (quest.end_npc != npc_id)
			return QuestStop::NPC;

		if (!end_items_met(quest, count))
			return QuestStop::ITEM;

		for (const auto& requirement : quest.end_prerequisites)
			if (status(requirement.first) != requirement.second)
				return QuestStop::QUEST;

		if (!end_info_met(quest, info_record))
			return QuestStop::INFO;

		if (!end_mobs_met(quest, progress))
			return QuestStop::MOB;

		if (meso < quest.end_meso)
			return QuestStop::MESO;

		return pet_stop(quest.end_pets, first_pet);
	}

	// Whether the player's fame, items, skills, worn items and map meet a
	// quest's start requirements, as v83 checks them (FUN_00721163), with
	// `first_pet` the item of the first pet out (0 for none). `held`
	// gives how many of an item the player holds, plus one if a copy is worn,
	// and `worn` whether a copy is worn. `has_skill` is v83's loose test: any
	// skill not hidden from the skill book counts, and a hidden one once it
	// has a record (FUN_007619b0).
	template<class Held, class HasSkill, class Worn>
	bool start_holdings_met(const QuestInfo& quest, int16_t fame, Held held, HasSkill has_skill, Worn worn,
		int32_t mapid, int32_t first_pet = 0)
	{
		if (pet_stop(quest.pets, first_pet) != QuestStop::NONE)
			return false;

		if (!std::all_of(quest.worn_all.begin(), quest.worn_all.end(), worn))
			return false;

		if (!quest.worn_any.empty() && std::none_of(quest.worn_any.begin(), quest.worn_any.end(), worn))
			return false;

		if (!quest.field_enter.empty() &&
			std::find(quest.field_enter.begin(), quest.field_enter.end(), mapid) == quest.field_enter.end())
			return false;

		// v83 compares fame unsigned, so negative fame passes.
		if (quest.min_fame != 0 && static_cast<uint16_t>(fame) < static_cast<uint16_t>(quest.min_fame))
			return false;

		for (const auto& requirement : quest.held_items)
		{
			int32_t count = held(requirement.first);

			if (requirement.second > 0 ? count < requirement.second :
				requirement.second == 0 ? count != 0 : count > -requirement.second)
				return false;
		}

		return std::all_of(quest.skills.begin(), quest.skills.end(),
			[&](const auto& requirement) { return has_skill(requirement.first) == requirement.second; });
	}

	// Whether a completed quest can be started again: it is repeatable and
	// its interval has passed since `completed_at`. Both times are FILETIMEs
	// (FUN_00721163).
	inline bool repeat_due(const QuestInfo& quest, int64_t completed_at, int64_t now)
	{
		constexpr int64_t TICKS_PER_MINUTE = 600000000;

		return quest.repeat_interval >= 0 && completed_at + quest.repeat_interval * TICKS_PER_MINUTE <= now;
	}

	// A Check.nx date, yyyymmddhh, as a FILETIME. v83 parses it to the hour
	// (FUN_00720d26) and compares it with its clock moved to the server's,
	// which for RustMS is UTC.
	inline int64_t quest_date(int64_t date)
	{
		int64_t year = date / 1000000;
		int64_t month = date / 10000 % 100;
		int64_t day = date / 100 % 100;
		int64_t hour = date % 100;

		// Howard Hinnant's days_from_civil.
		year -= month <= 2 ? 1 : 0;
		int64_t era = (year >= 0 ? year : year - 399) / 400;
		int64_t yoe = year - era * 400;
		int64_t doy = (153 * (month > 2 ? month - 3 : month + 9) + 2) / 5 + day - 1;
		int64_t doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
		int64_t days = era * 146097 + doe - 719468;

		return 116444736000000000 + (days * 86400 + hour * 3600) * 10000000;
	}

	// `holds` checks start_holdings_met for the player and, for a completed
	// quest, repeat_due.
	template<class Status, class Holds>
	bool quest_available(const QuestInfo& quest, int16_t job, int16_t level,
		Status status, Holds holds, bool all_levels = false)
	{
		return status(quest.id) != 1 && (status(quest.id) == 0 || quest.repeat_interval >= 0) &&
			(quest.jobs.empty() || std::find(quest.jobs.begin(), quest.jobs.end(), job) != quest.jobs.end()) &&
			(all_levels || (level >= quest.min_level && level <= quest.max_level)) &&
			std::all_of(quest.prerequisites.begin(), quest.prerequisites.end(),
				[&](const auto& requirement) { return status(requirement.first) == requirement.second; }) &&
			holds(quest);
	}

	// The icon over an NPC, as v83 chooses it (FUN_006d1779). The NPC's quests
	// are those it starts or finishes, less party quests (1200-1399). A started
	// quest it can finish now (kills and items in) gives the closed book;
	// otherwise a quest it can start gives the bulb, and a started quest the
	// open book. Kept free of graphics so the choice can be tested.
	// `progress` gives a started quest's progress string (its kill counts),
	// and `holds` checks start_holdings_met.
	template<class Status, class Count, class Progress, class Holds>
	QuestMarker select_quest_marker(int32_t npc_id, int16_t job,
		const std::vector<QuestInfo>& quests, Status status, int16_t level, Count count,
		Progress progress, Holds holds)
	{
		bool available = false, in_progress = false;

		for (const auto& quest : quests)
		{
			if (quest.start_npc != npc_id && quest.end_npc != npc_id)
				continue;

			if (quest.id >= 1200 && quest.id < 1400)
				continue;

			if (status(quest.id) == 1)
			{
				if ((quest.end_npc == 0 || quest.end_npc == npc_id) &&
					end_items_met(quest, count) && end_mobs_met(quest, progress(quest.id)))
				{
					std::string record = progress(quest.end_info_record);

					if (end_info_met(quest, &record))
						return QuestMarker::READY;
				}

				in_progress = true;
			}
			else if ((quest.start_npc == 0 || quest.start_npc == npc_id) &&
				quest_available(quest, job, level, status, holds))
			{
				available = true;
			}
		}

		if (available)
			return QuestMarker::AVAILABLE;

		return in_progress ? QuestMarker::IN_PROGRESS : QuestMarker::NONE;
	}

	// The quests an NPC lists when clicked, in v83's groups (FUN_006d1779):
	// started quests it can complete now, quests it can start, and its other
	// started quests. Party quests (1200-1399) are left out.
	struct QuestMenu
	{
		std::vector<int16_t> completable;
		std::vector<int16_t> available;
		std::vector<int16_t> in_progress;
		// All of the NPC's quests, and those completed for good. Party
		// quests count only in the first.
		size_t quests = 0;
		size_t finished = 0;

		bool empty() const
		{
			return completable.empty() && available.empty() && in_progress.empty();
		}
	};

	// `can_start` checks Check.img/<id>/0, `can_complete` Check.img/<id>/1
	// with this NPC.
	template<class Status, class CanStart, class CanComplete>
	QuestMenu select_quest_menu(int32_t npc_id, const std::vector<QuestInfo>& quests, Status status,
		CanStart can_start, CanComplete can_complete)
	{
		QuestMenu menu;

		for (const auto& quest : quests)
		{
			if (quest.start_npc != npc_id && quest.end_npc != npc_id)
				continue;

			menu.quests++;

			if (quest.id >= 1200 && quest.id < 1400)
				continue;

			int16_t state = status(quest.id);

			if (state == 1)
				(can_complete(quest) ? menu.completable : menu.in_progress).push_back(quest.id);
			else if (quest.start_npc == npc_id && can_start(quest))
				menu.available.push_back(quest.id);
			else if (state == 2)
				menu.finished++;
		}

		return menu;
	}

	// A quest's line in that menu (FUN_00724bac): its type in bold red, its
	// name, and for started quests whether it can be completed.
	inline std::string quest_menu_line(int32_t choice, const QuestInfo& quest, bool started, bool completable)
	{
		std::string label = quest.name;

		if (started)
			label += completable ? " (Pre-completion enabled)" : " (In Progress)";

		if (!quest.type.empty())
			label = "#r#e" + quest.type + "#d#n" + label;

		return "#d#L" + std::to_string(choice) + "# " + label + "#l#k\r\n";
	}
}
