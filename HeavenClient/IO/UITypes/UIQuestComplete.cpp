#include "UIQuestComplete.h"

#include "UIQuestLog.h"

#include "../UI.h"
#include "../UiControl.h"

#include "../Components/MapleButton.h"

#include "../../Audio/Audio.h"
#include "../../Character/QuestCatalog.h"
#include "../../Constants.h"
#include "../../Util/Misc.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// The first notice's left edge and how far its top sits above the
		// bottom of the screen; later ones sit higher, over it (FUN_00523408,
		// FadePlace). The text is laid out from footage (FRS4wNJWWIE, 2:00).
		constexpr int16_t LEFT = 464, ABOVE_BOTTOM = 97;
		constexpr int16_t TEXT_X = 24, NAME_WIDTH = 104;
		const Point<int16_t> ICON(6, 12), HEADING(TEXT_X, 2), NAME(TEXT_X, 15);
		const Point<int16_t> OK_AT(133, 5), CANCEL_AT(133, 19);
		// How long from appearing until it fades out unanswered.
		constexpr uint32_t LIFETIME = 30000;
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
		for (const Notice& notice : notices)
		{
			Point<int16_t> at = position + offset(notice);
			float opacity = notice.life.opacity();

			background.draw(DrawArgument(at, opacity));
			icon.draw(DrawArgument(at + ICON, opacity));
			heading.draw(DrawArgument(at + HEADING, opacity));
			notice.name.draw(DrawArgument(at + NAME, opacity));
		}

		// Only the top notice can be reached; the others' buttons lie under it.
		if (!notices.empty())
		{
			float opacity = notices.back().life.opacity();

			for (auto& [id, button] : buttons)
				static_cast<MapleButton*>(button.get())->set_opacity(opacity);
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

		// Quests handed in or forfeited take their notice with them; the rest
		// fade, and those below a notice that leaves move down. The server's
		// notices may be for quests the catalog lacks, as the explorer titles.
		for (auto it = notices.begin(); it != notices.end();)
		{
			it->life.update(Constants::TIMESTEP, ui_control::state().locked);

			if (it->life.gone() || !quests.is_started(it->id))
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
			push_notice(id);
			changed = true;
		}

		// Notices of other kinds coming and going move these too.
		if (changed || (!notices.empty() && notices.back().life.raised() != buttons_raised))
			rebuild();

		UIElement::update();
	}

	void UIQuestComplete::add_notice(int16_t id)
	{
		push_notice(id);
		rebuild();
	}

	void UIQuestComplete::push_notice(int16_t id)
	{
		std::string title = nl::nx::quest["QuestInfo.img"][std::to_string(id)]["name"];
		Text name(Text::A12M, Text::LEFT, Color::Name::WHITE, title.empty() ? std::to_string(id) : title);
		string_format::format_with_ellipsis(name, NAME_WIDTH);

		notices.erase(std::remove_if(notices.begin(), notices.end(), [&](const Notice& n) { return n.id == id; }),
			notices.end());
		notices.push_back({ id, std::move(name), FadeNotice(LIFETIME) });
		Sound(Sound::Name::INVITE).play();
	}

	void UIQuestComplete::rebuild()
	{
		buttons.clear();

		nl::node fade = NxFiles::ui_classic["UIWindow.img"]["FadeYesNo"];

		if (notices.empty())
			return;

		buttons_raised = notices.back().life.raised();
		Point<int16_t> at = offset(notices.back());

		buttons[OK] = std::make_unique<MapleButton>(fade["BtOK"], at + OK_AT);
		buttons[CANCEL] = std::make_unique<MapleButton>(fade["BtCancel"], at + CANCEL_AT);
	}

	Point<int16_t> UIQuestComplete::offset(const Notice& notice) const
	{
		return Point<int16_t>(0, -notice.life.raised());
	}

	void UIQuestComplete::update_screen(int16_t, int16_t new_height)
	{
		position = Point<int16_t>(LEFT, new_height - ABOVE_BOTTOM);
	}

	bool UIQuestComplete::is_in_range(Point<int16_t> cursorpos) const
	{
		if (notices.empty())
			return false;

		Point<int16_t> top = position + offset(notices.back());
		Point<int16_t> bottom = position + offset(notices.front()) + Point<int16_t>(WIDTH, HEIGHT);

		return Rectangle<int16_t>(top, bottom).contains(cursorpos);
	}

	Button::State UIQuestComplete::button_pressed(uint16_t buttonid)
	{
		// One press can arrive twice, and a notice fading out ignores more;
		// act on the first. One LOCK_UI holds back is out of reach.
		if (notices.empty() || notices.back().life.closed() || notices.back().life.waiting())
			return Button::State::NORMAL;

		Notice& notice = notices.back();
		notice.life.close();

		if (buttonid == OK)
		{
			auto log = UI::get().get_element<UIQuestLog>();

			if (!log || !log->is_active())
				UI::get().emplace<UIQuestLog>(quests);

			if (auto opened = UI::get().get_element<UIQuestLog>())
				opened->show_quest(notice.id);
		}

		return Button::State::NORMAL;
	}

	UIElement::Type UIQuestComplete::get_type() const
	{
		return TYPE;
	}
}
