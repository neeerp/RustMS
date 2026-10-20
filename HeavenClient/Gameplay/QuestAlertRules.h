#pragma once

#include <algorithm>
#include <cstdint>
#include <set>
#include <vector>

// The quest alert, the bulb over the player's face: which auto quests it offers, and when it lights and sounds, as
// v83 keeps them in CWvsContext (FUN_00a08af6, FUN_00a08c7e, FUN_00a08d30, FUN_0095ced9; report 201-quest-alert).
namespace ms::quest_alert
{
	// v83 checks the autoComplete quests this often (FUN_0094a144), and opens an autoAccept quest that didn't start
	// again after this long (FUN_00a03350).
	constexpr int32_t INTERVAL = 10000;

	enum class Bulb
	{
		NONE,
		APPEAR,
		DEFAULT
	};

	struct State
	{
		// autoStart quests that can be started (ctx+0x3754).
		std::set<int16_t> startable;
		// autoAccept quests opened already (ctx+0x376c).
		std::set<int16_t> accepted;
		// Started autoPreComplete quests (ctx+0x3784).
		std::set<int16_t> pre_complete;
		// autoComplete quests that can be completed, in the order found (ctx+0x37a8).
		std::vector<int16_t> completable;
		// A quest was added since the alert last sounded (ctx+0x379c, ctx+0x37bc).
		bool new_start = false;
		bool new_complete = false;
		// When an autoAccept quest last opened (ctx+0x37a4).
		int32_t accepted_at = 0;
		Bulb bulb = Bulb::NONE;
	};

	// An autoStart quest checked against its start requirements. Returns whether it should open by itself: an
	// autoAccept quest the first time it passes. Any other that passes joins the bulb's quests, and with `notify`
	// sounds the alert if it is new.
	inline bool offer(State &state, int16_t quest, bool passes, bool auto_accept, bool notify, int32_t now)
	{
		if (!passes)
		{
			state.startable.erase(quest);
			state.accepted.erase(quest);

			return false;
		}

		if (auto_accept)
		{
			if (!state.accepted.insert(quest).second)
				return false;

			state.accepted_at = now;

			return true;
		}

		if (state.startable.insert(quest).second && notify)
			state.new_start = true;

		return false;
	}

	// An autoComplete quest checked against its end requirements with its end NPC. One that newly passes sounds the
	// alert.
	inline void check_complete(State &state, int16_t quest, bool passes)
	{
		auto found = std::find(state.completable.begin(), state.completable.end(), quest);

		if (!passes)
		{
			if (found != state.completable.end())
				state.completable.erase(found);
		}
		else if (found == state.completable.end())
		{
			state.completable.push_back(quest);
			state.new_complete = true;
		}
	}

	// A quest's record changed to `record` (0 forfeited, 1 started, 2 completed; FUN_00a20f4c, FUN_00a216ab).
	inline void on_record(State &state, int16_t quest, int8_t record, bool auto_pre_complete)
	{
		state.startable.erase(quest);

		if (record == 1)
		{
			if (auto_pre_complete)
				state.pre_complete.insert(quest);

			return;
		}

		state.pre_complete.erase(quest);
		state.completable.erase(std::remove(state.completable.begin(), state.completable.end(), quest),
								state.completable.end());
	}

	// The player entered a field (FUN_00a086d5): the startable and pre-complete quests are found again, and the bulb
	// starts over.
	inline void rescan(State &state)
	{
		state.startable.clear();
		state.pre_complete.clear();
		state.bulb = Bulb::NONE;
	}

	// Whether an autoAccept quest was opened over 10 s ago, so they should all be forgotten and found again.
	inline bool reoffer_due(const State &state, int32_t now)
	{
		return !state.accepted.empty() && now - state.accepted_at > INTERVAL;
	}

	inline bool shown(const State &state)
	{
		return !state.startable.empty() || !state.pre_complete.empty() || !state.completable.empty();
	}

	// One frame of the bulb. Returns whether the alert sounds: a line in the chat, the sound, and the bulb appearing
	// again unless it is now. While the UI is locked the alert waits. Appear plays once, then Default loops.
	inline bool update(State &state, bool ui_locked, bool appear_done)
	{
		if (!shown(state))
		{
			state.bulb = Bulb::NONE;
			state.new_start = false;
			state.new_complete = false;

			return false;
		}

		if (!ui_locked && (state.new_start || state.new_complete))
		{
			state.bulb = Bulb::APPEAR;
			state.new_start = false;
			state.new_complete = false;

			return true;
		}

		if (state.bulb == Bulb::NONE || (state.bulb == Bulb::APPEAR && appear_done))
			state.bulb = Bulb::DEFAULT;

		return false;
	}

	struct Entry
	{
		int16_t quest;
		// UtilDlgEx/list<header>: 3 "QUEST THAT CAN BE COMPLETED", 1 "QUEST AVAILABLE", 0 "QUEST IN PROGRESS".
		int header;
	};

	// The bulb's quests, in the order its menu lists them (FUN_0095d260).
	inline std::vector<Entry> menu(const State &state)
	{
		std::vector<Entry> entries;

		for (int16_t quest : state.completable)
			entries.push_back({ quest, 3 });

		for (int16_t quest : state.startable)
			entries.push_back({ quest, 1 });

		for (int16_t quest : state.pre_complete)
			entries.push_back({ quest, 0 });

		return entries;
	}
}
