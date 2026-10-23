// The Blacklist tab of UIUserList, as the client lays it out (0x918a9b) and
// edits it (0x919214, 0x919506; RE in v83ref b089 and b090).
#include "UIUserList.h"
#include "UserListLayout.h"

#include "UINotice.h"

#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Configuration.h"

#include "../../Character/Blacklist.h"
#include "../../Data/CurseData.h"
#include "../../Gameplay/Stage.h"
#include "../../Util/NxFiles.h"

#include "../../Net/Packets/MiniRoomPackets.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		using namespace UserListLayout;

		// The tab is drawn on a canvas at this spot, and never scrolls: 26
		// names fill 13 rows.
		constexpr Point<int16_t> BLACK_CANVAS = Point<int16_t>(6, 53);
		// Under the header, two names to a row strip.
		constexpr int16_t BLACK_ROWS = 27;
		constexpr int16_t BLACK_COLUMN = 128;
		// Only the top of a strip picks its name.
		constexpr int16_t BLACK_HIT = 14;
		constexpr Point<int16_t> BLACK_NAME = Point<int16_t>(25, 4);
		// "n/26", by its right edge on the header.
		constexpr Point<int16_t> BLACK_COUNT = Point<int16_t>(261, 8);
		// Its tooltip's area runs 20 either side of it.
		constexpr int16_t BLACK_COUNT_MARGIN = 20;
		constexpr int16_t BLACK_COUNT_TOP = 3;
		constexpr int16_t BLACK_COUNT_BOTTOM = 25;
		// By their top left.
		constexpr Point<int16_t> BLACK_ADD = Point<int16_t>(60, 355);
		constexpr Point<int16_t> BLACK_DELETE = Point<int16_t>(180, 355);

		// StringPool 0x2d5, 0x335, 0x2d7, 0x2d6, 0x2d8 and 0x2b2.
		constexpr const char* BLACK_PROMPT = "Please enter the name of the character you'd like to add to the Blacklist.";
		constexpr const char* BLACK_TOO_SHORT = "Please enter at least 4 characters.";
		constexpr const char* BLACK_INVALID = "This character name does not exist\\nPlease check again";
		constexpr const char* BLACK_SELF = "You may not enter yourself in the Blacklist";
		constexpr const char* BLACK_LISTED = "This name has already been registered\\nPlease check again";
		constexpr const char* BLACK_COUNT_HELP = "Currently Registered Users / Maximum Capacity";

		std::vector<std::string> saved_blacklist()
		{
			return Blacklist::parse(Setting<BlackList>::get().load());
		}

		// Kept at once, as v83 writes its registry on each change (0x49e46c).
		void save_blacklist(const std::vector<std::string>& names)
		{
			Setting<BlackList>::get().save(Blacklist::format(names));
			Configuration::get().save();
		}

		void black_notice(const char* text)
		{
			UI::get().emplace<UIOk>(text, [](bool) {});
		}
	}

	bool Blacklist::listed(const std::string& name)
	{
		return listed(saved_blacklist(), name);
	}

	void UIUserList::init_blacklist()
	{
		nl::node BlackList = NxFiles::ui_classic["UIWindow.img"]["UserList"]["BlackList"];

		black_header = BlackList["blacklist0"];
		black_row = BlackList["blacklist1"];
		black_end = BlackList["blacklist2"];

		for (auto [id, name, pos] : { std::tuple(Buttons::BT_BLACK_ADD, "BtAdd", BLACK_ADD),
									  std::tuple(Buttons::BT_BLACK_DELETE, "BtDelete", BLACK_DELETE) })
		{
			nl::node button = BlackList[name];
			buttons[id] = std::make_unique<MapleButton>(button, pos + Texture(button["normal"]["0"]).get_origin());
		}

		black_count = Text(Text::Font::A12M, Text::Alignment::RIGHT, Color::Name::WHITE);
		black_fill = ColorBox(BLACK_COLUMN, ROW, Color::Name::PARCELSELECT, 1.0f);
	}

	void UIUserList::rebuild_blacklist()
	{
		black_names = Blacklist::sorted(saved_blacklist());
		black_texts.clear();

		for (size_t i = 0; i < black_names.size(); i++)
			black_texts.emplace_back(Text::Font::A12M, Text::Alignment::LEFT,
									 i == black_picked ? Color::Name::WHITE : Color::Name::BLACK, black_names[i]);

		black_count.change_text(std::to_string(black_names.size()) + "/" + std::to_string(Blacklist::MAX));
	}

	void UIUserList::draw_blacklist() const
	{
		Point<int16_t> canvas = position + BLACK_CANVAS;

		black_header.draw(canvas);
		black_count.draw(canvas + BLACK_COUNT + Point<int16_t>(0, CLIENT_TEXT));

		int16_t y = BLACK_ROWS;

		for (size_t i = 0; i < black_texts.size(); i++)
		{
			int16_t x = i % 2 == 0 ? 0 : BLACK_COLUMN;

			if (i % 2 == 0)
				black_row.draw(canvas + Point<int16_t>(0, y));

			if (i == black_picked)
				black_fill.draw(canvas + Point<int16_t>(x + 1, y));

			black_texts[i].draw(canvas + Point<int16_t>(x, y) + BLACK_NAME + Point<int16_t>(0, CLIENT_TEXT));

			if (i % 2 == 1)
				y += ROW;
		}

		if (black_texts.size() % 2 == 1)
			y += ROW;

		// An empty list says so, on no strip of its own.
		if (black_texts.empty())
		{
			none.draw(canvas + Point<int16_t>(0, y) + BLACK_NAME + Point<int16_t>(0, CLIENT_TEXT));
			y += ROW;
		}

		black_end.draw(canvas + Point<int16_t>(0, y));
	}

	Cursor::State UIUserList::blacklist_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		Point<int16_t> local = cursorpos - position - BLACK_CANVAS;
		int16_t count_left = BLACK_COUNT.x() - black_count.width() - BLACK_COUNT_MARGIN;

		if (Rectangle<int16_t>(count_left, BLACK_COUNT.x() + BLACK_COUNT_MARGIN, BLACK_COUNT_TOP, BLACK_COUNT_BOTTOM)
				.contains(local))
			show_tip(BLACK_COUNT_HELP);
		else
			show_tip(nullptr);

		if (!clicked || !UIElement::is_in_range(cursorpos))
			return UIElement::send_cursor(clicked, cursorpos);

		for (auto& [id, button] : buttons)
			if (button->is_active() && button->bounds(position).contains(cursorpos))
				return UIElement::send_cursor(clicked, cursorpos);

		// Any other click picks the name under it, or none (0x919619).
		size_t picked = NO_BLACK_PICK;

		for (size_t i = 0; i < black_names.size(); i++)
		{
			int16_t x = i % 2 == 0 ? 0 : BLACK_COLUMN;
			int16_t y = BLACK_ROWS + static_cast<int16_t>(i / 2) * ROW;

			if (Rectangle<int16_t>(x, x + BLACK_COLUMN, y, y + BLACK_HIT).contains(local))
				picked = i;
		}

		if (picked != black_picked)
		{
			black_picked = picked;
			rebuild_blacklist();
		}

		return Cursor::State::IDLE;
	}

	Button::State UIUserList::blacklist_button(uint16_t buttonid)
	{
		if (buttonid == Buttons::BT_BLACK_ADD)
			prompt_blacklist();
		else if (buttonid == Buttons::BT_BLACK_DELETE)
			delete_from_blacklist();

		return Button::State::NORMAL;
	}

	void UIUserList::prompt_blacklist()
	{
		UI::get().emplace<UIEnterText>(BLACK_PROMPT, [](const std::string& name)
		{
			auto userlist = UI::get().get_element<UIUserList>();

			if (name.size() < Blacklist::MIN_NAME)
			{
				// The prompt stays open in v83; here it comes back.
				UI::get().emplace<UIOk>(BLACK_TOO_SHORT, [](bool)
				{
					if (auto userlist = UI::get().get_element<UIUserList>())
						userlist->prompt_blacklist();
				});
			}
			else if (userlist)
			{
				userlist->add_to_blacklist(name);
			}
		}, Blacklist::MAX_NAME, true);
	}

	void UIUserList::add_to_blacklist(const std::string& name)
	{
		std::vector<std::string> names = saved_blacklist();

		switch (Blacklist::check(names, name, Stage::get().get_player().get_name(), CurseData::in_name(name)))
		{
		case Blacklist::Refusal::INVALID:
			black_notice(BLACK_INVALID);
			return;
		case Blacklist::Refusal::SELF:
			black_notice(BLACK_SELF);
			return;
		case Blacklist::Refusal::LISTED:
			black_notice(BLACK_LISTED);
			return;
		case Blacklist::Refusal::FULL:
			return;
		case Blacklist::Refusal::NONE:
			break;
		}

		// The pick stays on its place in the list, whoever is there now.
		names.push_back(name);
		save_blacklist(names);
		rebuild_blacklist();
		update_buttons();

		if (Blacklist::live)
			ShopBlacklistChangePacket(true, name).dispatch();
	}

	void UIUserList::delete_from_blacklist()
	{
		// Nothing picked, nothing deleted; no confirmation.
		if (black_picked >= black_names.size())
			return;

		std::string name = black_names[black_picked];
		std::vector<std::string> names = saved_blacklist();

		Blacklist::remove(names, name);
		save_blacklist(names);
		rebuild_blacklist();
		update_buttons();

		if (Blacklist::live)
			ShopBlacklistChangePacket(false, name).dispatch();
	}

	void UIUserList::update_blacklist_buttons()
	{
		// Delete needs no pick (0x91919b).
		enable(Buttons::BT_BLACK_ADD, black_names.size() < Blacklist::MAX);
		enable(Buttons::BT_BLACK_DELETE, !black_names.empty());
	}
}
