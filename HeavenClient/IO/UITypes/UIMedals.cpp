#include "UIMedals.h"
#include "UINotice.h"
#include "UIQuestLog.h"

#include "../UI.h"

#include "../Components/AreaButton.h"
#include "../Components/MapleButton.h"

#include "../../Constants.h"
#include "../../Character/Inventory/ItemExpiry.h"
#include "../../Character/QuestCatalog.h"
#include "../../Character/QuestRequirements.h"
#include "../../Data/ItemData.h"
#include "../../Gameplay/QuestConversation.h"
#include "../../Gameplay/Stage.h"
#include "../../Graphics/GraphicsGL.h"
#include "../../Net/Packets/QuestPackets.h"
#include "../../Util/Misc.h"
#include "../../Util/NpcTextMarkup.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

#include <algorithm>
#include <cstdio>
#include <limits>

namespace ms
{
	namespace
	{
		// The list (FUN_00843bf6). HC's text stands lower than v83's at the
		// same point, so its positions are moved up by TEXT_RAISE.
		constexpr int16_t WIDTH = 260;
		constexpr int16_t HEIGHT = 374;
		constexpr int16_t TEXT_RAISE = 6;
		constexpr Point<int16_t> TAB_POS = Point<int16_t>(4, 24);
		constexpr int16_t TAB_WIDTH = 50;
		constexpr int16_t TAB_HEIGHT = 19;
		// The categories' tabs: a header bar, or a title's panel.
		constexpr int16_t LIST_TOP = 48;
		constexpr int16_t LIST_BOTTOM = 339;
		constexpr int16_t ROW_X = 7;
		constexpr int16_t ROW_WIDTH = 230;
		constexpr int16_t HEADER_HEIGHT = 18;
		constexpr int16_t HEADER_PITCH = 23;
		// A 2 pixel gap above each header but the first shown.
		constexpr int16_t HEADER_GAP = 2;
		constexpr int16_t ENTRY_HEIGHT = 46;
		constexpr int16_t ENTRY_PITCH = 49;
		constexpr Point<int16_t> FOLD_BOX = Point<int16_t>(10, 3);
		constexpr Point<int16_t> HEADER_TEXT = Point<int16_t>(29, 4);
		// In a title's panel: the placeholder frame, the medal's icon by its
		// bottom left, its name and the line under it.
		constexpr Point<int16_t> FRAME_POS = Point<int16_t>(13, 5);
		constexpr Point<int16_t> ICON_POS = Point<int16_t>(15, 39);
		constexpr Point<int16_t> NAME_POS = Point<int16_t>(55, 8);
		constexpr Point<int16_t> LINE_POS = Point<int16_t>(55, 27);
		constexpr int16_t TEXT_WIDTH_MAX = 170;
		// The Basic tab: the count over a strip, then the titles' panels.
		constexpr Point<int16_t> STRIP_POS = Point<int16_t>(3, 45);
		constexpr Point<int16_t> COUNT_POS = Point<int16_t>(58, 57);
		constexpr int16_t BASIC_TOP = 96;
		constexpr int16_t BASIC_BOTTOM = 338;
		// The scroll bar (0x843a93) moves a line at a step, and stops with
		// the last four lines shown.
		constexpr int16_t SLIDER_X = 239;
		constexpr Range<int16_t> SLIDER = Range<int16_t>(LIST_TOP, LIST_TOP + 290);
		constexpr Range<int16_t> BASIC_SLIDER = Range<int16_t>(93, 93 + 245);
		constexpr int16_t LAST_LINES = 4;
		// Clicks this low on the list, off a line, close the details.
		constexpr int16_t DESELECT_TOP = 40;
		// The details (FUN_0083dba2), from their own origin. They slide out
		// to the list's right.
		constexpr int16_t DETAIL_X = 259;
		constexpr int16_t DETAIL_WIDTH = 305;
		constexpr Point<int16_t> HIDE_POS = Point<int16_t>(285, 6);
		constexpr Point<int16_t> TITLE_POS = Point<int16_t>(35, 42);
		constexpr int16_t TITLE_WIDTH = 150;
		constexpr Point<int16_t> REQUIREMENT_POS = Point<int16_t>(23, 75);
		// The start NPC standing, by its origin.
		constexpr Point<int16_t> PORTRAIT_POS = Point<int16_t>(242, 110);
		constexpr Point<int16_t> TEXT_POS = Point<int16_t>(17, 123);
		constexpr int16_t TEXT_WIDTH = 263;
		constexpr Range<int16_t> TEXT_VIEW = Range<int16_t>(124, 124 + 213);
		constexpr int16_t TEXT_STEP = 18;
		constexpr int16_t DETAIL_SLIDER_X = 283;
		constexpr Range<int16_t> DETAIL_SLIDER = Range<int16_t>(123, 123 + 215);
		constexpr Point<int16_t> CHALLENGE_POS = Point<int16_t>(184, 350);
		constexpr Point<int16_t> FORFEIT_POS = Point<int16_t>(243, 350);
		// Only Challenge titles can be taken on or given up here.
		constexpr int32_t CHALLENGE_CATEGORY = 2;

		// A title as the data has it.
		struct TitleQuest
		{
			int16_t quest;
			int32_t category;
			int32_t min_level;
			int32_t medal;
		};

		// The titles: QuestInfo.img's area 51, without the chained ones
		// (FUN_008423e2).
		const std::vector<TitleQuest>& title_quests()
		{
			static const std::vector<TitleQuest> titles = []()
			{
				std::vector<TitleQuest> found;

				for (nl::node info : nl::nx::quest["QuestInfo.img"])
				{
					if (info["area"].get_integer() != QuestCatalog::TITLE_AREA || info["parent"])
						continue;

					int16_t id = string_conversion::or_zero<int16_t>(info.name());
					int32_t category = static_cast<int32_t>(info["medalCategory"].get_integer());

					// Category 4 is listed only once obtained, in Basic.
					if (id == 0 || category < 0 || category > medal_list::NUM_TABS - 1)
						continue;

					int32_t level = nl::nx::quest["Check.img"][info.name()]["0"]["lvmin"];
					found.push_back({ id, category, level, info["viewMedalItem"] });
				}

				return found;
			}();

			return titles;
		}

		const TitleQuest* title_quest(int16_t id)
		{
			for (const TitleQuest& title : title_quests())
				if (title.quest == id)
					return &title;

			return nullptr;
		}

		// The start check's job ids in the data's order: by number, where the
		// NX file sorts their names as text.
		std::vector<int32_t> start_jobs(nl::node start)
		{
			std::vector<std::pair<int32_t, int32_t>> allowed;

			for (auto job : start["job"])
				allowed.emplace_back(string_conversion::or_zero<int32_t>(job.name()), static_cast<int32_t>(job.get_integer()));

			std::sort(allowed.begin(), allowed.end());

			std::vector<int32_t> ids;

			for (const auto& [order, job] : allowed)
				ids.push_back(job);

			return ids;
		}

		// Who may start it, as the quest window words it (FUN_0087fdc9).
		std::string level_line(nl::node start)
		{
			return QuestRequirements::levels(static_cast<bool>(start), static_cast<int32_t>(start["lvmin"].get_integer()),
				static_cast<int32_t>(start["lvmax"].get_integer()));
		}

		Text cut(Text::Font font, Color::Name color, const std::string& text, int16_t width)
		{
			Text out(font, Text::Alignment::LEFT, color, text, 0, false);
			string_format::format_with_ellipsis(out, width);

			return out;
		}

		std::string header_text(medal_list::Group group, int16_t count)
		{
			const char* name = group == medal_list::CHALLENGED ? "Title being challenged" : "Obtainable Title";

			return std::string(name) + " (" + std::to_string(count) + ")";
		}
	}

	UIMedals::UIMedals(const QuestLog& quests) : UIDragElement<PosMEDALS>(), questlog(quests)
	{
		nl::node Title = NxFiles::ui_classic["UIWindow.img"]["Title"];
		nl::node Basic = NxFiles::ui_classic["Basic.img"];

		background = Title["backgrnd"];
		strip = Title["backgrnd2"];
		detail_background = Title["backgrnd3"];
		row = Title["backgrnd4"];
		frame = Title["icon1"];
		worn_frame = Title["icon2"];
		fold_boxes[0] = Basic["BtMin"]["normal"]["0"];
		fold_boxes[1] = Basic["BtMax"]["normal"]["0"];

		for (uint8_t i = 0; i < medal_list::NUM_TABS; i++)
		{
			tabs_enabled[i] = Title["Tab"]["enabled"][i];
			tabs_disabled[i] = Title["Tab"]["disabled"][i];
			buttons[TAB0 + i] = std::make_unique<AreaButton>(TAB_POS + Point<int16_t>(TAB_WIDTH * i, 0),
				Point<int16_t>(TAB_WIDTH, TAB_HEIGHT));
			buttons[TAB0 + i]->set_tab();
		}

		Point<int16_t> detail = Point<int16_t>(DETAIL_X, 0);

		buttons[HIDE] = std::make_unique<MapleButton>(Basic["BtHide"], detail + HIDE_POS);
		buttons[CHALLENGE] = std::make_unique<MapleButton>(Title["BtOK"], detail + CHALLENGE_POS);
		buttons[FORFEIT] = std::make_unique<MapleButton>(Title["BtGiveup"], detail + FORFEIT_POS);

		detail_slider = Slider(Slider::Type::CLASSIC, DETAIL_SLIDER, DETAIL_X + DETAIL_SLIDER_X, 1, 1,
			[&](bool up)
			{
				int16_t view = TEXT_VIEW.length() - (TEXT_POS.y() - TEXT_VIEW.first());
				int16_t steps = std::max<int16_t>(0, (description.height() - view + TEXT_STEP - 1) / TEXT_STEP);
				detail_offset = std::clamp<int16_t>(detail_offset + (up ? -1 : 1), 0, steps);
			});

		seen_revision = std::numeric_limits<uint64_t>::max();
		change_tab(medal_list::BASIC);
		show_details(false);
		update();
	}

	void UIMedals::update_screen(int16_t width, int16_t height)
	{
		dimension = Point<int16_t>(details_open ? DETAIL_X + DETAIL_WIDTH : WIDTH, HEIGHT);
		dragarea = Point<int16_t>(dimension.x(), 20);

		if (position.x() < 0 || position.x() + dimension.x() > width)
			position.set_x(std::max<int16_t>(0, (width - dimension.x()) / 2));

		if (position.y() < 0 || position.y() + dimension.y() > height)
			position.set_y(std::max<int16_t>(0, (height - dimension.y()) / 2));
	}

	void UIMedals::update()
	{
		UIElement::update();
		worn_frame.update();

		Player& player = Stage::get().get_player();
		int16_t job = player.get_stats().get_stat(MapleStat::JOB);
		int16_t level = player.get_stats().get_stat(MapleStat::LEVEL);
		int32_t medal = player.get_look().get_equips().get_equip(EquipSlot::Id::MEDAL);

		if (seen_revision != questlog.revision() || seen_job != job || seen_level != level || worn != medal)
		{
			seen_revision = questlog.revision();
			seen_job = job;
			seen_level = level;
			worn = medal;
			refresh_list();
		}
	}

	// Each title's state (FUN_00a2a0be): one not started is listed only
	// while the player may start it.
	void UIMedals::refresh_list()
	{
		Player& player = Stage::get().get_player();
		const auto& stats = player.get_stats();
		std::vector<int16_t> startable = questlog.available_quests(stats.get_stat(MapleStat::JOB),
			stats.get_stat(MapleStat::LEVEL), [&](const QuestInfo& quest) { return player.quest_start_met(quest); });

		medals.clear();
		obtained = 0;

		for (const TitleQuest& title : title_quests())
		{
			const auto& completed = questlog.completed_quests();
			auto done = completed.find(title.quest);
			medal_list::State state;

			if (questlog.started_quests().count(title.quest))
				state = medal_list::CHALLENGING;
			else if (done != completed.end())
				state = medal_list::OBTAINED;
			else if (std::find(startable.begin(), startable.end(), title.quest) != startable.end())
				state = medal_list::OBTAINABLE;
			else
				continue;

			if (state == medal_list::OBTAINED)
				obtained++;

			medals.push_back({ title.quest, title.category, title.min_level, state,
				done != completed.end() ? done->second : 0 });
		}

		count = Text(Text::Font::A12B, Text::Alignment::LEFT, Color::Name::MINESHAFT,
			"Number of medals obtained: " + std::to_string(obtained), 0, false);

		lines.clear();

		for (const medal_list::Row& entry : medal_list::rows(medals, tab, folded[tab]))
		{
			Line line { entry, Text(), Text(), "", 0, 0 };

			if (entry.quest < 0)
			{
				line.name = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE,
					header_text(entry.group, entry.count), 0, false);
				lines.push_back(std::move(line));
				continue;
			}

			std::string id = std::to_string(entry.quest);
			nl::node start = nl::nx::quest["Check.img"][id]["0"];
			const TitleQuest* title = title_quest(entry.quest);
			std::string second;

			if (tab == medal_list::BASIC)
			{
				// "Ended at", the FILETIME as it is (FUN_00841b32).
				int64_t ended = questlog.completed_quests().at(entry.quest);

				if (ended > 0)
				{
					item_expiry::Time t = item_expiry::utc(ended);
					char text[48];
					std::snprintf(text, sizeof(text), "Ended at %d/%d/%d at %02d:00", t.year, t.month, t.day, t.hour);
					second = text;
				}
			}
			else
			{
				std::string levels = level_line(start);
				std::string jobs = QuestRequirements::jobs(start_jobs(start));
				second = levels.empty() ? jobs : jobs.empty() ? levels : levels + ", " + jobs;
			}

			line.name = cut(Text::Font::A12B, Color::Name::MEDALTEAL, nl::nx::quest["QuestInfo.img"][id]["name"], TEXT_WIDTH_MAX);
			line.requirement = cut(Text::Font::A11M, Color::Name::DUSTYGRAY, second, TEXT_WIDTH_MAX);
			line.second = second;
			line.medal = title ? title->medal : 0;
			lines.push_back(std::move(line));
		}

		offset = std::clamp<int16_t>(offset, 0, max_offset());
		layout();

		bool listed = std::any_of(lines.begin(), lines.end(), [&](const Line& line) { return line.row.quest == selected; });
		select(listed ? selected : -1);
	}

	// Where each line from `offset` on starts. v83 starts a row while its
	// top is above the bottom; here it must fit whole.
	void UIMedals::layout()
	{
		for (Line& line : lines)
			line.top = -1;

		if (tab == medal_list::BASIC)
		{
			for (size_t i = offset, k = 0; i < lines.size(); i++, k++)
			{
				int16_t top = BASIC_TOP + ENTRY_PITCH * static_cast<int16_t>(k);

				if (top + ENTRY_HEIGHT > BASIC_BOTTOM)
					break;

				lines[i].top = top;
			}
		}
		else
		{
			int16_t y = LIST_TOP;

			for (size_t i = offset; i < lines.size(); i++)
			{
				bool header = lines[i].row.quest < 0;

				if (header && i != static_cast<size_t>(offset))
					y += HEADER_GAP;

				if (y + (header ? HEADER_HEIGHT : ENTRY_HEIGHT) > LIST_BOTTOM)
					break;

				lines[i].top = y;
				y += header ? HEADER_PITCH : ENTRY_PITCH;
			}
		}

		slider.setrows(offset, 1, max_offset() + 1);
	}

	int16_t UIMedals::max_offset() const
	{
		return std::max<int16_t>(0, static_cast<int16_t>(lines.size()) - LAST_LINES);
	}

	void UIMedals::select(int16_t id)
	{
		auto found = std::find_if(medals.begin(), medals.end(), [&](const medal_list::Medal& m) { return m.quest == id; });

		if (id < 0 || found == medals.end())
		{
			selected = -1;
			show_details(false);
			return;
		}

		// The same title redrawn, as when its progress moves, keeps its place.
		if (id != selected || found->state != selected_state)
			detail_offset = 0;

		selected = id;
		selected_state = found->state;

		std::string name = std::to_string(id);
		nl::node start = nl::nx::quest["Check.img"][name]["0"];

		// Under the name, the line the list has.
		auto line = std::find_if(lines.begin(), lines.end(), [&](const Line& l) { return l.row.quest == id; });

		title = cut(Text::Font::A12B, Color::Name::WHITE, nl::nx::quest["QuestInfo.img"][name]["name"], TITLE_WIDTH);
		requirement = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE,
			line != lines.end() ? line->second : "", 0, false);

		int32_t start_npc = start["npc"];
		portrait = nl::nx::npc[string_format::extend_id(start_npc, 7) + ".img"]["stand"]["0"];

		write_details();
		show_details(true);
	}

	// What the title's quest says in its state: offered, started or done.
	void UIMedals::write_details()
	{
		std::string text = nl::nx::quest["QuestInfo.img"][std::to_string(selected)][std::to_string(selected_state)];

		NpcTextMarkup::Lookups lookups {
			[]() { return Stage::get().get_player().get_name(); },
			[](int32_t id) { return std::string(nl::nx::string["Npc.img"][std::to_string(id)]["name"]); },
			[](int32_t id) { return ItemData::get(id).get_name(); },
			[](int32_t id) { return std::string(nl::nx::string["Mob.img"][std::to_string(id)]["name"]); },
			[](int32_t id) { return NxHelper::Map::get_map_info_by_id(id).name; }
		};

		description = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK,
			NpcTextMarkup::expand(quest_text(text, selected), lookups), TEXT_WIDTH);

		int16_t view = TEXT_VIEW.length() - (TEXT_POS.y() - TEXT_VIEW.first());
		int16_t steps = std::max<int16_t>(0, (description.height() - view + TEXT_STEP - 1) / TEXT_STEP);
		detail_offset = std::min(detail_offset, steps);
		detail_slider.setrows(detail_offset, 1, steps + 1);
	}

	void UIMedals::show_details(bool open)
	{
		details_open = open && selected >= 0;

		const TitleQuest* quest = details_open ? title_quest(selected) : nullptr;
		bool challenge = quest && quest->category == CHALLENGE_CATEGORY;

		buttons[HIDE]->set_active(details_open);
		buttons[CHALLENGE]->set_active(challenge && selected_state == medal_list::OBTAINABLE);
		buttons[FORFEIT]->set_active(challenge && selected_state == medal_list::CHALLENGING);

		update_screen(Constants::Constants::get().get_viewwidth(), Constants::Constants::get().get_viewheight());
	}

	void UIMedals::draw(float alpha) const
	{
		Point<int16_t> raise = Point<int16_t>(0, -TEXT_RAISE);

		// The details lie behind the list, slid out from under it.
		if (details_open)
		{
			Point<int16_t> detail = position + Point<int16_t>(DETAIL_X, 0);

			detail_background.draw(detail);
			portrait.draw(detail + PORTRAIT_POS);
			title.draw(detail + TITLE_POS + raise);
			requirement.draw(detail + REQUIREMENT_POS + raise);

			Range<int16_t> clip = Range<int16_t>(detail.y() + TEXT_VIEW.first(), detail.y() + TEXT_VIEW.second());
			description.draw(detail + TEXT_POS + raise - Point<int16_t>(0, detail_offset * TEXT_STEP), clip);
		}

		background.draw(position);

		// The tab shown is the pink one, as in period footage.
		for (uint8_t i = 0; i < medal_list::NUM_TABS; i++)
			(i == tab ? tabs_enabled : tabs_disabled)[i].draw(position + TAB_POS + Point<int16_t>(TAB_WIDTH * i, 0));

		if (tab == medal_list::BASIC)
		{
			strip.draw(position + STRIP_POS);
			count.draw(position + COUNT_POS + raise);
		}

		for (const Line& line : lines)
		{
			if (line.top < 0)
				continue;

			Point<int16_t> top = position + Point<int16_t>(0, line.top);

			if (line.row.quest < 0)
			{
				GraphicsGL::get().drawrectangle(top.x() + ROW_X, top.y(), ROW_WIDTH, HEADER_HEIGHT,
					0.62f, 0.71f, 0.81f, 1.0f);
				fold_boxes[(folded[tab] >> line.row.group) & 1].draw(top + FOLD_BOX);
				line.name.draw(top + HEADER_TEXT + raise);
				continue;
			}

			row.draw(top + Point<int16_t>(ROW_X, 0));

			// Not yet challenged, a title shows only the frame; one being
			// challenged without a medal shows nothing (FUN_00843bf6).
			bool obtainable = tab != medal_list::BASIC && line.row.group == medal_list::AVAILABLE;

			if (line.medal != 0 && !obtainable)
				ItemData::get(line.medal).get_icon(false).draw(top + ICON_POS);
			else if (obtainable || tab == medal_list::BASIC)
				frame.draw(top + FRAME_POS);

			if (tab == medal_list::BASIC && line.medal != 0 && line.medal == worn)
				worn_frame.draw(top + FRAME_POS, alpha);

			line.name.draw(top + NAME_POS + raise);
			line.requirement.draw(top + LINE_POS + raise);
		}

		slider.draw(position);

		if (details_open)
			detail_slider.draw(position);

		UIElement::draw_buttons(alpha);
	}

	void UIMedals::toggle_active()
	{
		if (active)
		{
			deactivate();
		}
		else
		{
			makeactive();
			seen_revision = std::numeric_limits<uint64_t>::max();
		}
	}

	void UIMedals::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (pressed && escape)
			deactivate();
	}

	void UIMedals::send_scroll(double yoffset)
	{
		if (details_open && cursor_relative.x() >= DETAIL_X)
			detail_slider.send_scroll(yoffset);
		else
			slider.send_scroll(yoffset);
	}

	const UIMedals::Line* UIMedals::line_at(Point<int16_t> cursor) const
	{
		if (cursor.x() < ROW_X || cursor.x() >= ROW_X + ROW_WIDTH)
			return nullptr;

		for (const Line& line : lines)
		{
			int16_t height = line.row.quest < 0 ? HEADER_HEIGHT : ENTRY_HEIGHT;

			if (line.top >= 0 && cursor.y() >= line.top && cursor.y() < line.top + height)
				return &line;
		}

		return nullptr;
	}

	Cursor::State UIMedals::send_cursor(bool clicking, Point<int16_t> cursorpos)
	{
		if (dragged)
			return UIDragElement::send_cursor(clicking, cursorpos);

		cursor_relative = cursorpos - position;
		// Only a press acts, not the button held while the mouse moves.
		bool pressed = clicking && UI::get().is_cursor_press();

		if (Cursor::State state = slider.send_cursor(cursor_relative, clicking))
			return state;

		if (details_open)
			if (Cursor::State state = detail_slider.send_cursor(cursor_relative, clicking))
				return state;

		const Line* line = line_at(cursor_relative);

		// A title's medal names itself under the cursor (FUN_008417ec).
		Point<int16_t> icon = line ? cursor_relative - Point<int16_t>(0, line->top) - FRAME_POS : Point<int16_t>(-1, -1);
		Point<int16_t> box = frame.get_dimensions();

		if (line && line->row.quest >= 0 && line->medal != 0 && icon.x() >= 0 && icon.y() >= 0 && icon.x() < box.x()
			&& icon.y() < box.y())
		{
			UI::get().show_text(Tooltip::Parent::MEDALS, ItemData::get(line->medal).get_name());
			tooltip = true;
		}
		else
		{
			clear_tooltip();
		}

		if (line)
		{
			if (!pressed)
				return clicking ? Cursor::State::CLICKING : Cursor::State::CANCLICK;

			if (line->row.quest < 0)
			{
				folded[tab] ^= 1 << line->row.group;
				refresh_list();
			}
			else
			{
				select(line->row.quest);
			}

			return Cursor::State::CLICKING;
		}

		// A click on the list off its rows closes the details (FUN_00841678).
		if (pressed && selected >= 0 && cursor_relative.x() < WIDTH && cursor_relative.y() >= DESELECT_TOP)
			select(-1);

		return UIDragElement::send_cursor(clicking, cursorpos);
	}

	void UIMedals::remove_cursor()
	{
		UIDragElement::remove_cursor();

		clear_tooltip();
	}

	void UIMedals::clear_tooltip()
	{
		if (!tooltip)
			return;

		tooltip = false;
		UI::get().clear_tooltip(Tooltip::Parent::MEDALS);
	}

	Button::State UIMedals::button_pressed(uint16_t buttonid)
	{
		if (buttonid < TAB0 + medal_list::NUM_TABS)
		{
			change_tab(static_cast<uint8_t>(buttonid - TAB0));
			return Button::State::IDENTITY;
		}

		switch (buttonid)
		{
			case HIDE:
				select(-1);
				break;
			case CHALLENGE:
				challenge();
				break;
			case FORFEIT:
				give_up();
				break;
			default:
				break;
		}

		return Button::State::NORMAL;
	}

	// The same as accepting it from its NPC (FUN_00a26e31).
	void UIMedals::challenge()
	{
		if (const QuestInfo* quest = QuestCatalog::get().find(selected))
			QuestConversation::open(selected, quest->start_npc);
	}

	// v83 asks first and sends the forfeit, as the quest log does (FUN_00a26ea7).
	void UIMedals::give_up()
	{
		int16_t id = selected;

		if (!questlog.started_quests().count(id))
			return;

		UI::get().emplace<UIYesNo>("Are you sure you want to quit?", [id](bool yes)
		{
			if (!yes)
				return;

			Stage::get().get_player().get_quests().untrack(id);
			QuestActionPacket(QuestActionPacket::FORFEIT, id, 0, std::nullopt).dispatch();
		});
	}

	void UIMedals::show_title(int16_t id)
	{
		refresh_list();

		auto found = std::find_if(medals.begin(), medals.end(), [&](const medal_list::Medal& m) { return m.quest == id; });

		if (found == medals.end())
			return;

		uint8_t tabid = found->state == medal_list::OBTAINED ? medal_list::BASIC : static_cast<uint8_t>(found->category + 1);
		folded[tabid] &= ~(1 << medal_list::group_of(found->state));
		change_tab(tabid);
		select(id);

		for (size_t i = 0; i < lines.size(); i++)
		{
			if (lines[i].row.quest == id && lines[i].top < 0)
			{
				offset = std::min<int16_t>(static_cast<int16_t>(i), max_offset());
				layout();
				break;
			}
		}
	}

	void UIMedals::change_tab(uint8_t tabid)
	{
		if (tabid != tab)
			offset = 0;

		buttons[TAB0 + tab]->set_state(Button::State::NORMAL);
		tab = tabid;
		buttons[TAB0 + tab]->set_state(Button::State::PRESSED);

		slider = Slider(Slider::Type::CLASSIC, tab == medal_list::BASIC ? BASIC_SLIDER : SLIDER, SLIDER_X, 1, 0,
			[&](bool up) { offset = std::clamp<int16_t>(offset + (up ? -1 : 1), 0, max_offset()); layout(); });

		selected = -1;
		refresh_list();
	}

	UIElement::Type UIMedals::get_type() const
	{
		return TYPE;
	}
}
