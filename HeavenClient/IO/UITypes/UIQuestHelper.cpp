#include "UIQuestHelper.h"
#include "UIPartyHP.h"

#include "../Components/MapleButton.h"

#include "../../Character/Inventory/Inventory.h"
#include "../../Character/QuestCatalog.h"
#include "../../Constants.h"
#include "../../Data/ItemData.h"
#include "../../Gameplay/Stage.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// Where the quest lines start, from footage.
		constexpr int16_t TEXT_X = 9;
		// Room left after a quest's name for its untrack button.
		constexpr int16_t UNTRACK_ROOM = 24;
		// Between a requirement's name and its count.
		constexpr int16_t COUNT_GAP = 12;
		// The original's windows drag by their top 25 pixels.
		constexpr int16_t DRAG_HEIGHT = 25;
		// The saved position before the helper is first moved
		constexpr Point<int16_t> UNSET = Point<int16_t>(-1, -1);
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
		else
		{
			top.draw(position);

			int16_t rows = height() - TOP - BOTTOM;
			center.draw(DrawArgument(position + Point<int16_t>(0, TOP), Point<int16_t>(WIDTH, rows)));
			bottom.draw(position + Point<int16_t>(0, TOP + rows));

			int16_t y = TOP;

			for (const auto& entry : entries)
			{
				entry.name.draw(position + Point<int16_t>(TEXT_X, y - 6));
				y += entry.name.height() > ROW ? (entry.name.height() + ROW - 1) / ROW * ROW : ROW;

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

		std::vector<std::pair<int16_t, std::vector<QuestObjective>>> current;
		const auto& started = quests.started_quests();

		for (int16_t id : quests.tracked())
		{
			const QuestInfo* quest = QuestCatalog::get().find(id);
			auto progress = started.find(id);

			if (!quest || progress == started.end())
				continue;

			current.emplace_back(id, quest_objectives(quest->end_mobs, quest->end_items, progress->second,
				[&](int32_t item_id) { return static_cast<int32_t>(inventory.get_total_item_count(item_id)); }));
		}

		if (current != shown)
		{
			shown = std::move(current);
			rebuild();
		}

		UIElement::update();
	}

	void UIQuestHelper::rebuild()
	{
		title = Text(Text::A12M, Text::LEFT, Color::Name::BLACK,
			"Quest Helper (" + std::to_string(shown.size()) + "/" + std::to_string(QuestLog::MAX_TRACKED) + ")");
		entries.clear();

		for (auto it = buttons.lower_bound(UNTRACK); it != buttons.end();)
			it = buttons.erase(it);

		int16_t y = TOP;

		for (const auto& [id, objectives] : shown)
		{
			const QuestInfo* quest = QuestCatalog::get().find(id);
			Entry entry { id, Text(Text::A12B, Text::LEFT, Color::Name::BLACK, quest ? quest->name : std::to_string(id),
				WIDTH - TEXT_X - UNTRACK_ROOM) };

			// The untrack button follows the name.
			Point<int16_t> end = entry.name.endoffset();
			uint16_t untrack = UNTRACK + static_cast<uint16_t>(entries.size());
			buttons[untrack] = std::make_unique<MapleButton>(NxFiles::ui_classic["Basic.img"]["BtClose2"],
				Point<int16_t>(TEXT_X + end.x() + 4, y + end.y() - 2));
			buttons[untrack]->set_active(!minimized);

			y += entry.name.height() > ROW ? (entry.name.height() + ROW - 1) / ROW * ROW : ROW;

			for (const auto& objective : objectives)
			{
				std::string name = objective.mob
					? nl::nx::string["Mob.img"][std::to_string(objective.id)]["name"].get_string()
					: ItemData::get(objective.id).get_name();

				// Only the count you have is coloured, once it is enough.
				entry.requirements.push_back({
					Text(Text::A12M, Text::LEFT, Color::Name::DARKGREY, name),
					Text(Text::A12M, Text::LEFT, objective.met() ? Color::Name::HIPPIEGREEN : Color::Name::DARKGREY,
						std::to_string(objective.have)),
					Text(Text::A12M, Text::LEFT, Color::Name::DARKGREY, "/" + std::to_string(objective.need))
				});
				y += ROW;
			}

			entries.push_back(std::move(entry));
		}

		dimension = Point<int16_t>(WIDTH, height());
	}

	int16_t UIQuestHelper::height() const
	{
		if (minimized)
			return TOP_MINIMIZED;

		int16_t rows = 0;

		for (const auto& entry : entries)
		{
			rows += entry.name.height() > ROW ? (entry.name.height() + ROW - 1) / ROW * ROW : ROW;
			rows += static_cast<int16_t>(entry.requirements.size()) * ROW;
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
		Cursor::State state = UIDragElement::send_cursor(clicked, cursorpos);

		if (dragged)
			moved = true;

		return state;
	}

	bool UIQuestHelper::indragrange(Point<int16_t> cursorpos) const
	{
		Rectangle<int16_t> top(position, position + Point<int16_t>(WIDTH, std::min(DRAG_HEIGHT, height())));

		if (!top.contains(cursorpos))
			return false;

		for (const auto& [id, button] : buttons)
			if (button->is_active() && button->bounds(position).contains(cursorpos))
				return false;

		return true;
	}

	Button::State UIQuestHelper::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
			case AUTO:
				quests.set_auto_track(!quests.auto_tracks());
				break;
			case MINIMIZE:
			case MAXIMIZE:
				minimized = buttonid == MINIMIZE;
				buttons[MINIMIZE]->set_active(!minimized);
				buttons[MAXIMIZE]->set_active(minimized);

				for (auto it = buttons.lower_bound(UNTRACK); it != buttons.end(); ++it)
					it->second->set_active(!minimized);

				dimension = Point<int16_t>(WIDTH, height());
				break;
			case CLOSE:
				deactivate();
				break;
			default:
				// The buttons are rebuilt on the next update, not while the
				// caller walks them.
				if (buttonid >= UNTRACK && buttonid - UNTRACK < static_cast<int>(entries.size()))
					quests.untrack(entries[buttonid - UNTRACK].id);

				break;
		}

		return Button::State::NORMAL;
	}

	UIElement::Type UIQuestHelper::get_type() const
	{
		return TYPE;
	}
}
