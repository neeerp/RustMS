#include "UIQuestHelper.h"

#include "../Components/MapleButton.h"

#include "../../Character/Inventory/Inventory.h"
#include "../../Character/QuestCatalog.h"
#include "../../Constants.h"
#include "../../Data/ItemData.h"

#include <nlnx/nx.hpp>

namespace ms
{
	UIQuestHelper::UIQuestHelper(const QuestLog& quests, const Inventory& inventory) :
		UIElement(Point<int16_t>(), Point<int16_t>(WIDTH, 0)), quests(quests), inventory(inventory)
	{
		nl::node alarm = nl::nx::ui["UIWindow2.img"]["QuestAlarm"];

		top = alarm["backgrndmax"];
		top_minimized = alarm["backgrndmin"];
		center = alarm["backgrndcenter"];
		bottom = alarm["backgrndbottom"];
		icon = alarm["BtQ"]["normal"]["0"];

		buttons[MINIMIZE] = std::make_unique<MapleButton>(alarm["BtMin"]);
		buttons[MAXIMIZE] = std::make_unique<MapleButton>(alarm["BtMax"]);
		buttons[MAXIMIZE]->set_active(false);

		update_screen(Constants::Constants::get().get_viewwidth(), Constants::Constants::get().get_viewheight());
		rebuild();
	}

	void UIQuestHelper::draw(float alpha) const
	{
		if (quests.tracked().empty())
			return;

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

			for (const auto& line : lines)
			{
				line.text.draw(position + Point<int16_t>(10, y + 1));
				y += line.rows * ROW;
			}
		}

		icon.draw(position);
		title.draw(position + Point<int16_t>(22, 4));
		UIElement::draw_buttons(alpha);
	}

	void UIQuestHelper::update()
	{
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
		title = Text(Text::A11B, Text::LEFT, Color::Name::WHITE,
			"Quest Helper (" + std::to_string(shown.size()) + "/" + std::to_string(QuestLog::MAX_TRACKED) + ")");
		lines.clear();

		auto add = [&](Text::Font font, Color::Name color, const std::string& text)
		{
			Text line(font, Text::LEFT, color, text, WIDTH - 20);
			int16_t rows = std::max<int16_t>(1, (line.height() + ROW - 1) / ROW);
			lines.push_back({ std::move(line), rows });
		};

		for (const auto& [id, objectives] : shown)
		{
			const QuestInfo* quest = QuestCatalog::get().find(id);
			add(Text::A11B, Color::Name::SUPERNOVA, quest ? quest->name : std::to_string(id));

			for (const auto& objective : objectives)
			{
				std::string name = objective.mob
					? nl::nx::string["Mob.img"][std::to_string(objective.id)]["name"].get_string()
					: ItemData::get(objective.id).get_name();
				std::string count = std::to_string(objective.have) + "/" + std::to_string(objective.need);
				add(Text::A11M, objective.met() ? Color::Name::LIGHTGREEN : Color::Name::WHITE, "  " + name + " " + count);
			}
		}

		dimension = Point<int16_t>(WIDTH, height());
	}

	int16_t UIQuestHelper::height() const
	{
		if (quests.tracked().empty())
			return 0;

		if (minimized)
			return TOP_MINIMIZED;

		int16_t rows = 0;

		for (const auto& line : lines)
			rows += line.rows;

		return TOP + rows * ROW + BOTTOM;
	}

	void UIQuestHelper::update_screen(int16_t new_width, int16_t)
	{
		// Below the buff icons in the top-right corner.
		position = Point<int16_t>(new_width - WIDTH - 6, 92);
	}

	bool UIQuestHelper::is_in_range(Point<int16_t> cursorpos) const
	{
		if (quests.tracked().empty())
			return false;

		return Rectangle<int16_t>(position, position + Point<int16_t>(WIDTH, height())).contains(cursorpos);
	}

	Cursor::State UIQuestHelper::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		return UIElement::send_cursor(clicked, cursorpos);
	}

	Button::State UIQuestHelper::button_pressed(uint16_t buttonid)
	{
		minimized = buttonid == MINIMIZE;
		buttons[MINIMIZE]->set_active(!minimized);
		buttons[MAXIMIZE]->set_active(minimized);
		dimension = Point<int16_t>(WIDTH, height());

		return Button::State::NORMAL;
	}

	UIElement::Type UIQuestHelper::get_type() const
	{
		return TYPE;
	}
}
