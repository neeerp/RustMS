#include "UIQuestComplete.h"

#include "UIQuestLog.h"

#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Character/QuestCatalog.h"
#include "../../Constants.h"
#include "../../Util/Misc.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// Laid out from footage (FRS4wNJWWIE, 2:00): the box's left edge and
		// how far its top sits above the bottom of the screen.
		constexpr int16_t LEFT = 464, ABOVE_BOTTOM = 97;
		constexpr int16_t TEXT_X = 24, NAME_WIDTH = 104;
		const Point<int16_t> ICON(5, 12), HEADING(TEXT_X, 2), NAME(TEXT_X, 15);
		const Point<int16_t> OK_AT(133, 5), CANCEL_AT(133, 19);
		// Shown at most; the oldest gives way.
		constexpr size_t MAX_NOTICES = 4;
	}

	UIQuestComplete::UIQuestComplete(QuestLog& quests, const Inventory& inventory) :
		UIElement(Point<int16_t>(), Point<int16_t>(WIDTH, 0)), quests(quests), inventory(inventory)
	{
		nl::node fade = NxFiles::ui_classic["UIWindow.img"]["FadeYesNo"];

		background = fade["backgrnd4"];
		icon = fade["icon7"];
		heading = Text(Text::A12M, Text::LEFT, Color::Name::WHITE, "Quest Complete!");

		update_screen(Constants::Constants::get().get_viewwidth(), Constants::Constants::get().get_viewheight());
	}

	void UIQuestComplete::draw(float alpha) const
	{
		for (size_t i = 0; i < notices.size(); i++)
		{
			Point<int16_t> at = position - Point<int16_t>(0, static_cast<int16_t>((notices.size() - 1 - i) * HEIGHT));

			background.draw(at);
			icon.draw(at + ICON);
			heading.draw(at + HEADING);
			notices[i].name.draw(at + NAME);
		}

		UIElement::draw_buttons(alpha);
	}

	void UIQuestComplete::update()
	{
		std::map<int16_t, std::vector<QuestObjective>> started;

		for (const auto& [id, progress] : quests.started_quests())
			if (const QuestInfo* quest = QuestCatalog::get().find(id))
				started[id] = quest_objectives(quest->end_mobs, quest->end_items, progress,
					[&](int32_t item_id) { return static_cast<int32_t>(inventory.get_total_item_count(item_id)); });

		bool changed = false;

		// Quests handed in or forfeited take their notice with them.
		for (auto it = notices.begin(); it != notices.end();)
		{
			if (it->closed || !started.count(it->id))
			{
				it = notices.erase(it);
				changed = true;
			}
			else
			{
				++it;
			}
		}

		for (int16_t id : completion.update(started))
		{
			const QuestInfo* quest = QuestCatalog::get().find(id);
			Text name(Text::A12M, Text::LEFT, Color::Name::WHITE, quest ? quest->name : std::to_string(id));
			string_format::format_with_ellipsis(name, NAME_WIDTH);

			notices.erase(std::remove_if(notices.begin(), notices.end(), [&](const Notice& n) { return n.id == id; }),
				notices.end());
			notices.push_back({ id, std::move(name), false });

			if (notices.size() > MAX_NOTICES)
				notices.erase(notices.begin());

			changed = true;
		}

		if (changed)
			rebuild();

		UIElement::update();
	}

	void UIQuestComplete::rebuild()
	{
		buttons.clear();

		nl::node fade = NxFiles::ui_classic["UIWindow.img"]["FadeYesNo"];

		for (size_t i = 0; i < notices.size(); i++)
		{
			Point<int16_t> at(0, -static_cast<int16_t>((notices.size() - 1 - i) * HEIGHT));
			uint16_t base = static_cast<uint16_t>(2 * i);

			buttons[OK + base] = std::make_unique<MapleButton>(fade["BtOK"], at + OK_AT);
			buttons[CANCEL + base] = std::make_unique<MapleButton>(fade["BtCancel"], at + CANCEL_AT);
		}
	}

	void UIQuestComplete::update_screen(int16_t, int16_t new_height)
	{
		position = Point<int16_t>(LEFT, new_height - ABOVE_BOTTOM);
	}

	bool UIQuestComplete::is_in_range(Point<int16_t> cursorpos) const
	{
		if (notices.empty())
			return false;

		Point<int16_t> top = position - Point<int16_t>(0, static_cast<int16_t>((notices.size() - 1) * HEIGHT));

		return Rectangle<int16_t>(top, position + Point<int16_t>(WIDTH, HEIGHT)).contains(cursorpos);
	}

	Button::State UIQuestComplete::button_pressed(uint16_t buttonid)
	{
		size_t index = buttonid / 2;

		// One press can arrive twice; act on the first.
		if (index >= notices.size() || notices[index].closed)
			return Button::State::NORMAL;

		// The buttons are rebuilt on the next update, not while the caller
		// walks them.
		notices[index].closed = true;

		if (buttonid % 2 == OK)
		{
			auto log = UI::get().get_element<UIQuestLog>();

			if (!log || !log->is_active())
				UI::get().emplace<UIQuestLog>(quests);

			if (auto opened = UI::get().get_element<UIQuestLog>())
				opened->show_quest(notices[index].id);
		}

		return Button::State::NORMAL;
	}

	UIElement::Type UIQuestComplete::get_type() const
	{
		return TYPE;
	}
}
