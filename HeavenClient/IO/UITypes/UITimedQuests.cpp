#include "UITimedQuests.h"

#include "../UI.h"

#include "../../Constants.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

#include <algorithm>

namespace ms
{
	namespace
	{
		nl::node time_quest()
		{
			return NxFiles::ui_classic["UIWindow.img"]["Quest"]["TimeQuest"];
		}

		// The quest's timerUI, which names its clock.
		std::string timer_ui(int16_t quest)
		{
			return nl::nx::quest["QuestInfo.img"][std::to_string(quest)]["timerUI"].get_string();
		}

		// The clock's ringing frames (state 3), for a quest without a timerUI those of "default".
		nl::node ring(int16_t quest)
		{
			nl::node clocks = time_quest()["AlarmClock"];
			nl::node clock = clocks[timer_ui(quest)];

			return clock ? clock["3"] : clocks["default"]["3"];
		}

		// Only a timerUI of "default", as written, rings and booms (FUN_0088d2ce).
		bool alarmed(int16_t quest)
		{
			return timer_ui(quest) == "default";
		}

		// The quest's name over the time left (FUN_0088ef78).
		std::string tooltip(const timed_quests::Clock& clock)
		{
			std::string name = nl::nx::quest["QuestInfo.img"][std::to_string(clock.quest)]["name"].get_string();

			return name + "\\n" + timed_quests::time_left(clock.remaining);
		}

		constexpr int16_t SCREEN_WIDTH = 800;
		constexpr Point<int16_t> BAR = Point<int16_t>(52, 12);
		// The clock sits right of its bar, centred 5 px below the bar's top (FUN_0088ee1e).
		constexpr int16_t CLOCK_X = 52;
		constexpr int16_t CLOCK_Y = 5;
		constexpr int16_t CLOCK_HEIGHT = 25;
		constexpr int32_t TOOLTIP_MS = 500;
	}

	UITimedQuests::UITimedQuests() : hovered(0), since_tooltip(0)
	{
		nl::node bar = time_quest()["TimeBar"];
		backgrnd = bar["backgrnd"];

		for (size_t i = 0; i < 3; i++)
			gauge[i] = bar["TimeGage"][std::to_string(i)];

		update_screen(Constants::Constants::get().get_viewwidth(), Constants::Constants::get().get_viewheight());
	}

	UITimedQuests::~UITimedQuests()
	{
		for (Alarm& alarm : alarms)
			alarm.loop.stop();
	}

	void UITimedQuests::draw(float) const
	{
		for (const timed_quests::Clock& clock : timed_quests::shown())
		{
			Point<int16_t> at = origin() + Point<int16_t>(clock.x, clock.y);
			int32_t lit = timed_quests::segments(clock);

			// FUN_0088eb12: the bar fills from the right, one 1 px step of TimeGage/2 a step, with TimeGage/1
			// at its right end; TimeGage/0 closes its left end while it is full.
			backgrnd.draw(at);

			if (clock.remaining > 0)
			{
				int16_t cap = gauge[1].width();
				gauge[1].draw(at + Point<int16_t>(50 - cap, 2));

				for (int32_t i = 0; i < lit - cap && i < timed_quests::SEGMENTS - cap; i++)
					gauge[2].draw(at + Point<int16_t>(static_cast<int16_t>(49 - cap - i), 2));

				if (lit > timed_quests::SEGMENTS - 1)
					gauge[0].draw(at + Point<int16_t>(2, 2));
			}

			const Alarm* alarm = find_alarm(clock.serial);

			if (!alarm || alarm->faces.empty())
				continue;

			auto count = static_cast<int32_t>(alarm->faces.size());
			alarm->faces[timed_quests::frame(clock, count)].draw(at + Point<int16_t>(CLOCK_X, CLOCK_Y - CLOCK_HEIGHT / 2));
		}
	}

	void UITimedQuests::update()
	{
		std::vector<timed_quests::Clock>& clocks = timed_quests::shown();
		timed_quests::tick(clocks, Constants::TIMESTEP);

		for (const timed_quests::Clock& clock : clocks)
		{
			Alarm& alarm = alarm_for(clock);

			if (!alarm.alarmed)
				continue;

			// The alarm rings from the last ringing frame on (0x88df2f).
			auto count = static_cast<int32_t>(alarm.faces.size());

			if (!alarm.ringing && count >= 2 && timed_quests::frame(clock, count) == count - 2)
			{
				alarm.loop.play_loop();
				alarm.ringing = true;
			}
		}

		// The tooltip keeps up with the time (FUN_0088ea75).
		if (hovered)
		{
			since_tooltip += Constants::TIMESTEP;

			if (since_tooltip >= TOOLTIP_MS)
			{
				since_tooltip = 0;

				for (const timed_quests::Clock& clock : clocks)
					if (clock.serial == hovered)
						UI::get().show_text(Tooltip::Parent::TIMEDQUESTS, tooltip(clock));
			}
		}
	}

	void UITimedQuests::update_screen(int16_t, int16_t)
	{
		position = Point<int16_t>();
	}

	bool UITimedQuests::is_in_range(Point<int16_t> cursorpos) const
	{
		return clock_at(cursorpos) != nullptr;
	}

	Cursor::State UITimedQuests::send_cursor(bool, Point<int16_t> cursorpos)
	{
		if (const timed_quests::Clock* clock = clock_at(cursorpos))
		{
			if (clock->serial != hovered)
			{
				hovered = clock->serial;
				since_tooltip = 0;
				UI::get().show_text(Tooltip::Parent::TIMEDQUESTS, tooltip(*clock));
			}
		}
		else
		{
			remove_cursor();
		}

		return Cursor::State::IDLE;
	}

	void UITimedQuests::remove_cursor()
	{
		if (hovered)
			UI::get().clear_tooltip(Tooltip::Parent::TIMEDQUESTS);

		hovered = 0;
	}

	UIElement::Type UITimedQuests::get_type() const
	{
		return TYPE;
	}

	void UITimedQuests::taken_down(int16_t quest)
	{
		std::vector<timed_quests::Clock>& clocks = timed_quests::shown();
		auto clock = std::find_if(clocks.begin(), clocks.end(),
			[quest](const timed_quests::Clock& c) { return c.quest == quest; });

		if (clock == clocks.end())
			return;

		uint32_t serial = clock->serial;

		// FUN_0088da07: the alarm stops and the clock booms.
		Alarm& alarm = alarm_for(*clock);

		if (alarm.alarmed)
		{
			alarm.loop.stop();
			Sound(nl::nx::sound["Game.img"]["clockBoom"]).play();
		}

		alarms.erase(std::remove_if(alarms.begin(), alarms.end(),
			[serial](const Alarm& alarm) { return alarm.serial == serial; }), alarms.end());

		if (hovered == serial)
			remove_cursor();

		timed_quests::remove(clocks, quest);
	}

	const timed_quests::Clock* UITimedQuests::clock_at(Point<int16_t> cursorpos) const
	{
		for (const timed_quests::Clock& clock : timed_quests::shown())
		{
			Point<int16_t> at = origin() + Point<int16_t>(clock.x, clock.y);
			Rectangle<int16_t> bar(at, at + BAR);

			if (bar.contains(cursorpos))
				return &clock;
		}

		return nullptr;
	}

	const UITimedQuests::Alarm* UITimedQuests::find_alarm(uint32_t serial) const
	{
		for (const Alarm& alarm : alarms)
			if (alarm.serial == serial)
				return &alarm;

		return nullptr;
	}

	UITimedQuests::Alarm& UITimedQuests::alarm_for(const timed_quests::Clock& clock)
	{
		for (Alarm& alarm : alarms)
			if (alarm.serial == clock.serial)
				return alarm;

		Alarm alarm = { clock.serial, Sound(nl::nx::sound["Game.img"]["questCount"]), alarmed(clock.quest), false, {} };
		nl::node frames = ring(clock.quest);

		for (size_t i = 0; i < frames.size(); i++)
			alarm.faces.emplace_back(frames[std::to_string(i)]);

		alarms.push_back(std::move(alarm));

		return alarms.back();
	}

	Point<int16_t> UITimedQuests::origin() const
	{
		return Point<int16_t>(Constants::Constants::get().get_viewwidth() - SCREEN_WIDTH, 0);
	}
}
