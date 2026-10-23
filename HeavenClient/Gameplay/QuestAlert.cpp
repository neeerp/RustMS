#include "QuestAlert.h"

#include "QuestConversation.h"
#include "Stage.h"

#include "../Audio/Audio.h"
#include "../Character/QuestCatalog.h"
#include "../Constants.h"
#include "../IO/UI.h"
#include "../IO/UiControl.h"
#include "../IO/UITypes/UIChatBar.h"

#include <nlnx/nx.hpp>

#include <algorithm>

namespace ms
{
	namespace
	{
		// StringPool 0xfe1.
		constexpr const char *ARRIVED = "Quest Arrived!  Double-click the icon located on top of the Character.";
	}

	void QuestAlert::init()
	{
		nl::node src = nl::nx::effect["BasicEff.img"]["QuestAlert"];

		appear = src["Appear"];
		idle = src["Default"];
	}

	void QuestAlert::reset()
	{
		state = {};
		active = false;
		armed = false;
	}

	void QuestAlert::enter_field(int32_t id)
	{
		active = true;
		mapid = id;
		quest_alert::rescan(state);

		const auto &quests = QuestCatalog::get().quests();

		// The field's own quests sound the alert on every entry; the others light the bulb silently.
		for (const QuestInfo &quest : quests)
			if (quest.auto_start && std::find(quest.field_enter.begin(), quest.field_enter.end(), mapid) !=
										quest.field_enter.end())
				offer(quest, true);

		for (const QuestInfo &quest : quests)
			if (quest.auto_start && quest.normal_auto_start)
				offer(quest, false);

		for (const auto &[quest_id, progress] : Stage::get().get_player().get_quests().started_quests())
			if (const QuestInfo *quest = QuestCatalog::get().find(quest_id); quest && quest->auto_pre_complete)
				state.pre_complete.insert(quest_id);
	}

	void QuestAlert::stats_changed()
	{
		if (!active)
			return;

		for (const QuestInfo &quest : QuestCatalog::get().quests())
			if (quest.auto_start && quest.normal_auto_start)
				offer(quest, true);
	}

	void QuestAlert::record_changed(int16_t quest_id, int8_t record)
	{
		if (!active)
			return;

		const QuestInfo *quest = QuestCatalog::get().find(quest_id);
		quest_alert::on_record(state, quest_id, record, quest && quest->auto_pre_complete);
		stats_changed();
	}

	void QuestAlert::update()
	{
		if (!active)
			return;

		clock += Constants::TIMESTEP;
		since_check += Constants::TIMESTEP;

		if (since_check >= quest_alert::INTERVAL)
		{
			since_check = 0;

			for (const QuestInfo &quest : QuestCatalog::get().quests())
				if (quest.auto_complete)
					check_complete(quest);
		}

		// An autoAccept quest that didn't start is offered again (FUN_00a03350).
		if (quest_alert::reoffer_due(state, clock))
		{
			state.accepted.clear();
			enter_field(mapid);
		}

		quest_alert::Bulb before = state.bulb;
		bool appear_done = before == quest_alert::Bulb::APPEAR && appear.update();

		if (quest_alert::update(state, ui_control::state().locked, appear_done))
		{
			if (auto chatbar = UI::get().get_element<UIChatBar>())
				chatbar->send_chatline(ARRIVED, UIChatBar::LineType::GREY);

			Sound(Sound::Name::QUESTALERT).play(100);
		}

		if (state.bulb != before)
		{
			appear.reset();
			idle.reset();
		}
		else if (state.bulb == quest_alert::Bulb::DEFAULT)
		{
			idle.update();
		}
	}

	void QuestAlert::draw(Point<int16_t> face, float alpha) const
	{
		// It doesn't turn with the player (flip 0).
		if (state.bulb != quest_alert::Bulb::NONE)
			shown().draw(face, alpha);
	}

	void QuestAlert::send_cursor(Point<int16_t> face, Point<int16_t> cursor, bool pressed, bool held)
	{
		// The current frame's box (FUN_0095d614).
		const Animation &bulb = shown();
		Point<int16_t> left_top = face - bulb.get_origin();
		bool inside = state.bulb != quest_alert::Bulb::NONE &&
					  Rectangle<int16_t>(left_top, left_top + bulb.get_dimensions()).contains(cursor);

		if (pressed)
		{
			armed = inside;
		}
		else if (!held && armed)
		{
			armed = false;

			if (inside)
				open_menu();
		}
	}

	void QuestAlert::release_cursor()
	{
		armed = false;
	}

	void QuestAlert::offer(const QuestInfo &quest, bool notify)
	{
		if (quest_alert::offer(state, quest.id, QuestConversation::startable(quest), quest.auto_accept, notify, clock))
			QuestConversation::open(quest.id, quest.start_npc);
	}

	void QuestAlert::check_complete(const QuestInfo &quest)
	{
		quest_alert::check_complete(state, quest.id, QuestConversation::completable(quest, quest.end_npc));
	}

	void QuestAlert::open_menu()
	{
		QuestConversation::alert_menu(quest_alert::menu(state), [this](int16_t quest_id)
		{
			if (const QuestInfo *quest = QuestCatalog::get().find(quest_id))
			{
				// Opened with its start NPC, whatever the quest's state, then checked for completion as an
				// autoComplete quest is (FUN_0095d260).
				QuestConversation::open(quest_id, quest->start_npc);
				check_complete(*quest);
			}
		});
	}

	const Animation &QuestAlert::shown() const
	{
		return state.bulb == quest_alert::Bulb::APPEAR ? appear : idle;
	}
}
