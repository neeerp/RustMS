#include "UIQuestHelper.h"
#include "UIChatBar.h"
#include "UIPartyHP.h"
#include "UIQuestLog.h"

#include "../UI.h"

#include "../Components/MapleButton.h"
#include "../Components/TextFit.h"

#include "../../Audio/Audio.h"
#include "../../Character/Inventory/Inventory.h"
#include "../../Character/QuestCatalog.h"
#include "../../Constants.h"
#include "../../Data/ItemData.h"
#include "../../Gameplay/Stage.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

#include <algorithm>

namespace ms
{
	namespace
	{
		// Where the quest lines start, from footage.
		constexpr int16_t TEXT_X = 9;
		// A quest's name is cut to this, on one row (FUN_00885eb8).
		constexpr int16_t NAME_WIDTH = 170;
		// From the end of a quest's name to its untrack button.
		constexpr int16_t UNTRACK_GAP = 6;
		// A row's name and count fit in this, the gap between them aside.
		constexpr int16_t ROW_WIDTH = 175;
		// Between a requirement's name and its count.
		constexpr int16_t COUNT_GAP = 10;
		// Where the double click on a quest's rows ends.
		constexpr int16_t ROWS_RIGHT = 170;
		// The original's windows drag by their top 25 pixels.
		constexpr int16_t DRAG_HEIGHT = 25;
		// What a quest's untrack button says in the chat log (0x1047, the
		// original's spelling; FUN_00887ee8).
		constexpr const char* UNTRACKED = "] It has been excluded from the auto alarm and it will not be "
										  "automatically reigstered until you re log-on";
		// The saved position before the helper is first moved
		constexpr Point<int16_t> UNSET = Point<int16_t>(-1, -1);

		// The original colours the count you have by how far along it is
		// (FUN_00704c2c), in steps of a third.
		Color::Name count_color(const QuestObjective& objective)
		{
			int32_t percent = objective.have * 100 / std::max(objective.need, 1);

			if (percent <= 32)
				return Color::Name::BRIGHTRED;
			else if (percent <= 65)
				return Color::Name::WILDSTRAWBERRY;
			else if (percent < 100)
				return Color::Name::PIZAZZ;
			else
				return Color::Name::DEEPGREEN;
		}
	}

	UIQuestHelper::UIQuestHelper(QuestLog& quests, const Inventory& inventory) :
		quests(quests), inventory(inventory), moved(position != UNSET)
	{
		dimension = Point<int16_t>(WIDTH, 0);

		nl::node alarm = NxFiles::ui_classic["UIWindow.img"]["QuestAlarm"];
		nl::node basic = NxFiles::ui_classic["Basic.img"];

		top = alarm["backgrndmax"];
		top_minimized = alarm["backgrndmin"];
		center = alarm["backgrndcenter"];
		bottom = alarm["backgrndbottom"];
		icon = alarm["BtQ"]["normal"]["0"];

		// Laid out from footage, where the minimize and close buttons are the
		// blue ones from Basic.img.
		buttons[AUTO] = std::make_unique<MapleButton>(alarm["BtAuto"], Point<int16_t>(141, 4));
		buttons[MINIMIZE] = std::make_unique<MapleButton>(basic["BtMin"], Point<int16_t>(190, 4));
		buttons[MAXIMIZE] = std::make_unique<MapleButton>(basic["BtMax"], Point<int16_t>(190, 4));
		buttons[CLOSE] = std::make_unique<MapleButton>(basic["BtClose"], Point<int16_t>(203, 4));
		buttons[MAXIMIZE]->set_active(false);
		buttons[AUTO]->set_state(auto_rest());

		if (!moved)
			position = default_position();

		rebuild();
	}

	void UIQuestHelper::draw(float alpha) const
	{
		if (minimized)
		{
			top_minimized.draw(position);
		}
		else if (entries.empty())
		{
			// The window is as short as when minimized, so only the top of
			// the maximized art shows.
			top.draw(position, Rectangle<int16_t>(0, WIDTH, 0, TOP_MINIMIZED));
		}
		else
		{
			top.draw(position);

			int16_t rows = height() - TOP - BOTTOM;
			center.draw(DrawArgument(position + Point<int16_t>(0, TOP), Point<int16_t>(WIDTH, rows)));
			bottom.draw(position + Point<int16_t>(0, TOP + rows));

			int16_t y = TOP;

			for (const auto& entry : entries)
			{
				if (&entry != &entries.front())
					y += ROW;

				entry.name.draw(position + Point<int16_t>(TEXT_X, y - 6));
				y += ROW;

				for (const auto& requirement : entry.requirements)
				{
					Point<int16_t> at = position + Point<int16_t>(TEXT_X, y - 6);
					requirement.name.draw(at);
					at.shift_x(requirement.name.width() + COUNT_GAP);
					requirement.have.draw(at);
					at.shift_x(requirement.have.width());
					requirement.need.draw(at);
					y += ROW;
				}

				if (entry.meso)
				{
					Point<int16_t> at = position + Point<int16_t>(TEXT_X, y - 6);
					entry.meso_label.draw(at);
					at.shift_x(entry.meso_label.width() + COUNT_GAP);
					entry.meso_need.draw(at);
					y += ROW;
				}
			}
		}

		icon.draw(position + Point<int16_t>(5, 4));
		title.draw(position + Point<int16_t>(25, -5));
		UIElement::draw_buttons(alpha);
	}

	void UIQuestHelper::update()
	{
		if (!moved)
			position = default_position();

		std::vector<Shown> current;
		const auto& started = quests.started_quests();

		for (int16_t id : quests.tracked())
		{
			const QuestInfo* quest = QuestCatalog::get().find(id);
			auto progress = started.find(id);

			if (!quest || progress == started.end())
				continue;

			current.push_back({
				id,
				quest_helper_objectives(quest->end_mobs, quest->end_items, progress->second,
					[&](int32_t item_id) { return static_cast<int32_t>(inventory.get_total_item_count(item_id)); }),
				quest->end_meso,
				quest->end_meso != 0 ? inventory.get_meso() : 0,
				quests.added_automatically(id)
			});
		}

		if (current != shown)
		{
			shown = std::move(current);
			rebuild();

			// New quest data unfolds the window (FUN_00885d96).
			if (minimized)
				set_minimized(false);
		}

		UIElement::update();
	}

	void UIQuestHelper::toggle_minimized()
	{
		set_minimized(!minimized && !entries.empty());
	}

	void UIQuestHelper::set_minimized(bool folded)
	{
		minimized = folded;
		leave_names();
		buttons[MINIMIZE]->set_active(!minimized);
		buttons[MAXIMIZE]->set_active(minimized);

		for (auto it = buttons.lower_bound(UNTRACK); it != buttons.end(); ++it)
			it->second->set_active(!minimized);

		dimension = Point<int16_t>(WIDTH, height());
	}

	void UIQuestHelper::rebuild()
	{
		title = Text(Text::A12M, Text::LEFT, Color::Name::BLACK,
			"Quest Helper (" + std::to_string(shown.size()) + "/" + std::to_string(QuestLog::MAX_TRACKED) + ")");
		entries.clear();
		leave_names();

		for (auto it = buttons.lower_bound(UNTRACK); it != buttons.end();)
			it = buttons.erase(it);

		int16_t y = TOP;

		for (const auto& [id, objectives, meso_need, meso_have, automatic] : shown)
		{
			// As in the original (FUN_007052e6), a blank row separates quests.
			if (!entries.empty())
				y += ROW;

			const QuestInfo* quest = QuestCatalog::get().find(id);
			std::string name = quest ? quest->name : std::to_string(id);
			std::string cut = fit_text(Text::A12B, name, NAME_WIDTH);
			// A quest that came by itself is blue until it's done or kept
			// (FUN_00885eb8); its meso doesn't count.
			bool waiting = automatic &&
				!std::all_of(objectives.begin(), objectives.end(), [](const QuestObjective& o) { return o.met(); });

			Entry entry { id, Text(Text::A12B, Text::LEFT, waiting ? Color::Name::GIFTBLUE : Color::Name::BLACK, cut, 0, false) };
			int16_t width = entry.name.width();

			if (cut != name)
				entry.full_name = name;

			entry.name_area = Rectangle<int16_t>(TEXT_X, TEXT_X + width, y, y + ROW);

			// The untrack button follows the name.
			uint16_t untrack = UNTRACK + static_cast<uint16_t>(entries.size());
			buttons[untrack] = std::make_unique<MapleButton>(NxFiles::ui_classic["Basic.img"]["BtClose2"],
				Point<int16_t>(TEXT_X + width + UNTRACK_GAP, y - 2));
			buttons[untrack]->set_active(!minimized);

			y += ROW;
			int16_t rows_top = y;

			for (const auto& objective : objectives)
			{
				std::string wanted = objective.mob
					? nl::nx::string["Mob.img"][std::to_string(objective.id)]["name"].get_string()
					: ItemData::get(objective.id).get_name();

				// Only the count you have is coloured.
				Text have(Text::A12M, Text::LEFT, count_color(objective), std::to_string(objective.have));
				Text need(Text::A12M, Text::LEFT, Color::Name::BLACK, "/" + std::to_string(objective.need));

				// The name gives way, so the count stays in the window.
				std::string fitted = fit_text(Text::A12M, wanted, ROW_WIDTH - have.width() - need.width());

				entry.requirements.push_back({
					Text(Text::A12M, Text::LEFT, Color::Name::BLACK, fitted, 0, false), std::move(have), std::move(need)
				});
				y += ROW;
			}

			// As in the original (FUN_00885eb8), the end meso comes after the
			// mobs and items, even once you have it. It shows only what it
			// takes, red until you have that much (FUN_00887b21).
			if (meso_need != 0)
			{
				entry.meso = true;
				entry.meso_label = Text(Text::A12M, Text::LEFT, Color::Name::BLACK, "Meso");
				entry.meso_need = Text(Text::A12M, Text::LEFT,
					meso_have >= meso_need ? Color::Name::DEEPGREEN : Color::Name::BRIGHTRED, std::to_string(meso_need));
				y += ROW;
			}

			entry.rows_area = Rectangle<int16_t>(TEXT_X, ROWS_RIGHT, rows_top, y);
			entries.push_back(std::move(entry));
		}

		dimension = Point<int16_t>(WIDTH, height());
	}

	int16_t UIQuestHelper::height() const
	{
		// As in the original (FUN_007052e6), there is no body without quests.
		if (minimized || entries.empty())
			return TOP_MINIMIZED;

		int16_t rows = static_cast<int16_t>(entries.size() - 1) * ROW;

		for (const auto& entry : entries)
		{
			rows += ROW;
			rows += static_cast<int16_t>(entry.requirements.size()) * ROW;

			if (entry.meso)
				rows += ROW;
		}

		return TOP + rows + BOTTOM;
	}

	Point<int16_t> UIQuestHelper::default_position() const
	{
		// Flush with the top-right corner, below the party HP list when there
		// is one.
		return Point<int16_t>(Constants::Constants::get().get_viewwidth() - WIDTH,
			UIPartyHP::height_for(Stage::get().get_player().get_party()));
	}

	void UIQuestHelper::update_screen(int16_t, int16_t)
	{
		if (!moved)
			position = default_position();
	}

	bool UIQuestHelper::is_in_range(Point<int16_t> cursorpos) const
	{
		return Rectangle<int16_t>(position, position + Point<int16_t>(WIDTH, height())).contains(cursorpos);
	}

	Cursor::State UIQuestHelper::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		// A held auto button keeps the cursor, and the window stays put.
		if (auto_held && clicked)
			return send_cursor_to_auto(clicked, cursorpos);

		Cursor::State state = UIDragElement::send_cursor(clicked, cursorpos);

		if (dragged)
		{
			// A click that goes nowhere leaves it following the party list.
			moved = moved || drag_moved();
			leave_auto();
			leave_names();

			return state;
		}

		Cursor::State auto_state = send_cursor_to_auto(clicked, cursorpos);

		if (auto_state != Cursor::State::IDLE)
		{
			leave_names();

			return auto_state;
		}

		Cursor::State name_state = send_cursor_to_names(clicked, cursorpos);

		return name_state != Cursor::State::IDLE ? name_state : state;
	}

	void UIQuestHelper::remove_cursor()
	{
		UIDragElement::remove_cursor();
		leave_auto();
		leave_names();
	}

	Cursor::State UIQuestHelper::send_cursor_to_names(bool clicked, Point<int16_t> cursorpos)
	{
		Point<int16_t> at = cursorpos - position;
		auto entry = std::find_if(entries.begin(), entries.end(),
			[&](const Entry& e) { return e.name_area.contains(at); });

		if (minimized || entry == entries.end())
		{
			leave_names();

			return Cursor::State::IDLE;
		}

		if (clicked && UI::get().is_cursor_press())
		{
			// The original's second click is a double click and not a press,
			// so here the mark outlasts one more press.
			if (quests.added_automatically(entry->id))
			{
				quests.keep_tracked(entry->id);
				kept = entry->id;
				kept_presses = 2;
			}
			else if (kept == entry->id && kept_presses > 0)
			{
				kept_presses--;
			}
		}

		// The tip gives the whole name or, on a quest that may go, what a
		// click does (0xb3aba4).
		std::string tip = !entry->full_name.empty() ? entry->full_name
			: quests.added_automatically(entry->id) ? "Register" : "";

		if (tip.empty())
		{
			leave_names();
		}
		else
		{
			UI::get().show_text(Tooltip::Parent::TEXT, tip);
			name_tip = true;
		}

		return clicked ? Cursor::State::CLICKING : Cursor::State::CANCLICK2;
	}

	void UIQuestHelper::leave_names()
	{
		if (name_tip)
		{
			name_tip = false;
			UI::get().clear_tooltip(Tooltip::Parent::TEXT);
		}
	}

	void UIQuestHelper::doubleclick(Point<int16_t> cursorpos)
	{
		Point<int16_t> at = cursorpos - position;
		auto entry = std::find_if(entries.begin(), entries.end(),
			[&](const Entry& e) { return e.name_area.contains(at) || e.rows_area.contains(at); });

		if (minimized || entry == entries.end() || (entry->id == kept && kept_presses > 0))
			return;

		int16_t id = entry->id;
		auto log = UI::get().get_element<UIQuestLog>();

		// The quest already open closes the window (0x887c93).
		if (log && log->is_active() && log->selected_quest() == id)
		{
			log->deactivate();

			return;
		}

		if (!log || !log->is_active())
			UI::get().emplace<UIQuestLog>(quests);

		if (auto opened = UI::get().get_element<UIQuestLog>())
			opened->show_quest(id);
	}

	bool UIQuestHelper::own_cursor(uint16_t buttonid) const
	{
		return buttonid == AUTO;
	}

	Button::State UIQuestHelper::auto_rest() const
	{
		return quests.auto_tracks() ? Button::State::NORMAL : Button::State::DISABLED;
	}

	Cursor::State UIQuestHelper::send_cursor_to_auto(bool clicked, Point<int16_t> cursorpos)
	{
		Button& button = *buttons[AUTO];
		// The disabled look leaves it out of button_under.
		bool under = button.bounds(position).contains(cursorpos);

		if (auto_held)
		{
			if (clicked)
			{
				button.set_state(under ? Button::State::PRESSED : auto_rest());

				return Cursor::State::CLICKING;
			}

			auto_held = false;

			// It clicks as the mouse comes back up over it (FUN_00887f8a),
			// and is left hovered without a sound.
			if (under)
			{
				Sound(Sound::Name::BUTTONCLICK).play();
				quests.set_auto_track(!quests.auto_tracks());
				button.set_state(auto_rest() == Button::State::NORMAL ? Button::State::MOUSEOVER : auto_rest());
			}
		}

		if (!under)
		{
			leave_auto();

			return Cursor::State::IDLE;
		}

		if (clicked && UI::get().is_cursor_press())
		{
			auto_held = true;
			button.set_state(Button::State::PRESSED);

			if (auto_help)
			{
				auto_help = false;
				UI::get().clear_help(TYPE);
			}

			return Cursor::State::CLICKING;
		}

		// The mouse held down from elsewhere neither hovers nor presses it.
		if (clicked)
		{
			leave_auto();

			return Cursor::State::IDLE;
		}

		// Only the hover while it's on looks and sounds like one.
		if (quests.auto_tracks())
		{
			if (button.get_state() == Button::State::NORMAL)
				Sound(Sound::Name::BUTTONOVER).play();

			button.set_state(Button::State::MOUSEOVER);
		}
		else
		{
			button.set_state(Button::State::DISABLED);
		}

		// The tip offers what a click does, 0x1052 and 0x1054 while auto
		// alarm is off, 0x1053 and 0x1055 while it's on (FUN_00884a70).
		if (quests.auto_tracks())
			UI::get().show_help(TYPE, "Auto Alarm off",
				"When you click it, the quest will not register automatically even when the quest is in progress.");
		else
			UI::get().show_help(TYPE, "Auto Alarm on",
				"When you click it, quests in progress will register automatically and if it is not in progress "
				"for 10 minutes, it will disappear.");

		auto_help = true;

		return Cursor::State::CANCLICK;
	}

	void UIQuestHelper::leave_auto()
	{
		auto_held = false;
		buttons[AUTO]->set_state(auto_rest());

		if (auto_help)
		{
			auto_help = false;
			UI::get().clear_help(TYPE);
		}
	}

	bool UIQuestHelper::indragrange(Point<int16_t> cursorpos) const
	{
		Rectangle<int16_t> top(position, position + Point<int16_t>(WIDTH, std::min(DRAG_HEIGHT, height())));

		if (!top.contains(cursorpos))
			return false;

		for (const auto& [id, button] : buttons)
			if ((button->is_active() || id == AUTO) && button->bounds(position).contains(cursorpos))
				return false;

		return true;
	}

	Button::State UIQuestHelper::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
			case MINIMIZE:
			case MAXIMIZE:
				set_minimized(buttonid == MINIMIZE);
				break;
			case CLOSE:
				deactivate();
				break;
			default:
				// The buttons are rebuilt on the next update, not while the
				// caller walks them.
				if (buttonid >= UNTRACK && buttonid - UNTRACK < static_cast<int>(entries.size()))
				{
					int16_t id = entries[buttonid - UNTRACK].id;

					// A grey line names the quest left out (chat type 7).
					if (auto chatbar = UI::get().get_element<UIChatBar>())
					{
						const QuestInfo* quest = QuestCatalog::get().find(id);
						chatbar->send_chatline("[" + (quest ? quest->name : std::to_string(id)) + UNTRACKED,
							UIChatBar::LineType::GREY);
					}

					quests.untrack(id);
				}

				break;
		}

		return Button::State::NORMAL;
	}

	UIElement::Type UIQuestHelper::get_type() const
	{
		return TYPE;
	}
}
