// The Alliance ("Guild Union") tab of UIUserList and its rank panel, as the
// client lays them out (FUN_00912b49, FUN_00917bc4; RE in v83ref b025).
#include "UIUserList.h"
#include "UserListLayout.h"

#include "UIChatBar.h"
#include "UIMessenger.h"
#include "UINotice.h"

#include "../PartyCommands.h"
#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Constants.h"

#include "../../Character/Job.h"
#include "../../Data/CurseData.h"
#include "../../Gameplay/Stage.h"
#include "../../Graphics/GraphicsGL.h"
#include "../../Util/NxFiles.h"

#include "../../Net/Packets/GameplayPackets.h"
#include "../../Net/Packets/MessagingPackets.h"

#include <nlnx/nx.hpp>

#include <algorithm>

namespace ms
{
	namespace
	{
		using namespace UserListLayout;

		// The alliance's name, centred with room for an emblem it doesn't
		// have.
		constexpr int16_t UNION_NAME_Y = 5;
		// The member list is drawn on a canvas 18 pixels a row taller than
		// 572, rows counted as the client does (FUN_0091eee7), so it scrolls
		// further than it reaches.
		constexpr int16_t UNION_CANVAS = 572;
		constexpr size_t UNION_SECTIONS = 5;
		constexpr int16_t UNION_PER_PAGE = 5;
		// In a guild's header.
		constexpr Point<int16_t> UNION_MARK = Point<int16_t>(5, 6);
		constexpr Point<int16_t> UNION_FRAME = Point<int16_t>(5, 5);
		constexpr Point<int16_t> UNION_GUILD_NAME = Point<int16_t>(27, 8);
		constexpr Point<int16_t> UNION_COUNT = Point<int16_t>(155, 8);
		constexpr Point<int16_t> UNION_PAGE = Point<int16_t>(261, 8);
		constexpr Point<int16_t> UNION_FOLD = Point<int16_t>(142, 8);
		// By their top left, in the order of the alliance buttons in Buttons.
		// NX's BtWhere is never made.
		const std::pair<const char*, Point<int16_t>> UNION_BUTTONS[] = {
				{ "BtInvite", Point<int16_t>(9, 316) },
				{ "BtWithdraw", Point<int16_t>(80, 316) },
				{ "BtKick", Point<int16_t>(159, 336) },
				{ "BtUp", Point<int16_t>(9, 336) },
				{ "BtDown", Point<int16_t>(80, 336) },
				{ "BtChange", Point<int16_t>(159, 316) },
				{ "BtChat", Point<int16_t>(238, 336) },
				{ "BtInfo", Point<int16_t>(238, 363) },
				{ "BtWhisper", Point<int16_t>(238, 316) },
				{ "Btnotice", Point<int16_t>(288, 80) },
				{ "BtPartyInvite", Point<int16_t>(159, 363) }
		};
		// A guild's name in the invitation prompt.
		constexpr size_t UNION_GUILD_MIN = 4;
		constexpr size_t UNION_GUILD_MAX = 14;

		// BtInvite's prompt (0x914cfe, StringPool 0x1165). A name under
		// 4 letters gets 0x335 and the prompt stays open; one that is too
		// short once trimmed asks again.
		void prompt_union_invite()
		{
			UI::get().emplace<UIEnterText>(
					"Please enter the Guild name\\nto send the invitation for the Union.",
					[](const std::string& text)
					{
						std::string name = trim(text);

						if (name.size() < UNION_GUILD_MIN)
							UI::get().emplace<UIOk>(
									"Please enter at least " + std::to_string(UNION_GUILD_MIN) + " characters.",
									[](bool) { prompt_union_invite(); }
							);
						else
							InviteToAlliancePacket(name).dispatch();
					},
					UNION_GUILD_MAX, true, UNION_GUILD_MIN
			);
		}

		// The rank panel (FUN_0091e704), its rows under the guild panel's
		// first three pieces of art.
		const std::pair<const char*, Point<int16_t>> UNION_ART[] = {
				{ "guildgrade0", Point<int16_t>(6, 8) },
				{ "guildgrade1", Point<int16_t>(6, 45) },
				{ "guildgrade2", Point<int16_t>(6, 63) }
		};
		constexpr int16_t UNION_ROWS = 63;
	}

	void UIUserList::init_alliance()
	{
		nl::node UserList = NxFiles::ui_classic["UIWindow.img"]["UserList"];
		nl::node GuildUnion = UserList["GuildUnion"];

		for (uint16_t i = 0; i < std::size(UNION_BUTTONS); i++)
		{
			// Placed by the top left, whatever the art's origin, which BtChat,
			// BtWhisper and BtInfo set.
			auto [name, pos] = UNION_BUTTONS[i];
			nl::node button = GuildUnion[name];
			buttons[Buttons::BT_ALLIANCE_INVITE + i] = std::make_unique<MapleButton>(button,
																					  pos + Texture(button["normal"]["0"]).get_origin());
		}

		nl::node Basic = NxFiles::ui_classic["Basic.img"];
		nl::node itemSearch = NxFiles::ui_classic["UIWindow.img"]["itemSearch"];

		for (uint16_t s = 0; s < UNION_SECTIONS; s++)
		{
			uint16_t first = Buttons::BT_ALLIANCE_SECTION + s * SectionButton::SECTION_BUTTONS;

			buttons[first + SectionButton::FOLD] = std::make_unique<MapleButton>(Basic["BtMin"]);
			buttons[first + SectionButton::UNFOLD] = std::make_unique<MapleButton>(Basic["BtMax"]);
			buttons[first + SectionButton::PREVIOUS_PAGE] = std::make_unique<MapleButton>(itemSearch["BtLeft"]);
			buttons[first + SectionButton::NEXT_PAGE] = std::make_unique<MapleButton>(itemSearch["BtRight"]);
		}

		// Its own copies of the guild tab's art, but the frame for a guild
		// without a mark, which it takes from the guild tab's.
		union_header = GuildUnion["guildinfo0"];
		union_columns = GuildUnion["guildinfo2"];
		union_sorted = GuildUnion["guildinfo7"];
		union_row = GuildUnion["guildinfo3"];
		union_end = GuildUnion["guildinfo4"];
		union_notice_box = GuildUnion["guildinfo6"];

		for (AllianceSection& section : alliance_sections)
		{
			section.name = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE);
			section.count = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE);
			section.page_text = Text(Text::Font::A12M, Text::Alignment::RIGHT, Color::Name::WHITE);
		}

		alliance_name = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK);
		no_alliance = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK,
						   "This guild is not affiliated with any Guild Union.");
		union_notice_label = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, "Guild Union notice.");
		union_notice = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, "", 0, false);

		alliance_slider = Slider(Slider::Type::CLASSIC, GUILD_SLIDER, GUILD_SLIDER_X, 1, 0,
								 [&](bool up) { scroll_alliance(alliance_scroll + (up ? -1 : 1)); });

		nl::node GuildGrade = UserList["Guild"]["GuildGrade"];
		union_background = GuildUnion["backgrnd"];

		for (size_t i = 0; i < union_art.size(); i++)
			union_art[i] = GuildGrade[UNION_ART[i].first];

		for (auto [id, name, pos] : { std::tuple(Buttons::BT_UNION_MODIFY, "BtModf", GRADE_MODIFY),
									  std::tuple(Buttons::BT_UNION_SAVE, "BtSave", GRADE_SAVE) })
		{
			nl::node button = GuildGrade[name];
			panel_button_home[id] = UNION + pos + Texture(button["normal"]["0"]).get_origin();
			buttons[id] = std::make_unique<MapleButton>(button, panel_button_home[id]);
			buttons[id]->set_active(false);
		}
	}

	void UIUserList::rebuild_alliance()
	{
		shown_alliance = alliance.get_data();
		shown_allies = alliance.get_guilds();
		shown_own = guild.get_data();

		if (shown_alliance.notice != union_notice.get_text())
		{
			union_ticker_x = UNION_TICKER_START;
			union_ticker_time = 0;
		}

		alliance_name.change_text(shown_alliance.name);
		union_notice.change_text(shown_alliance.notice);

		bool joined = alliance.joined(guild);
		// The player's guild, then the others in the order they came.
		std::vector<const GuildData*> guilds;

		if (joined)
		{
			guilds.push_back(&shown_own);

			for (const GuildData& other : shown_allies)
				guilds.push_back(&other);
		}

		alliance_online = 0;
		int16_t y = 0;
		int16_t counted_rows = 0;

		for (size_t i = 0; i < alliance_sections.size(); i++)
		{
			AllianceSection& section = alliance_sections[i];
			section.shown = i < guilds.size();
			section.rows.clear();

			if (section.shown)
			{
				y = lay_out(section, *guilds[i], y);
				alliance_online += std::count_if(guilds[i]->members.begin(), guilds[i]->members.end(),
												 [](const GuildMember& member) { return member.online; });
			}
			else
			{
				section.guild_id = 0;
				section.page = 1;
				section.pages = 1;
			}

			// A page's rows and one more for each unfolded section, even
			// one with no guild; none without an alliance. An empty list
			// still draws its "none" row but counts only the one (0x91eee7).
			if (joined && section.expanded)
				counted_rows += static_cast<int16_t>(section.rows.size()) + 1;
		}

		alliance_height = ROW * counted_rows + UNION_CANVAS;

		scroll_alliance(alliance_scroll);

		if (union_slide.alive)
			rebuild_union();
	}

	int16_t UIUserList::lay_out(AllianceSection& section, const GuildData& data, int16_t y)
	{
		std::vector<const GuildMember*> members;

		// Online members only, though the count has everyone.
		for (const GuildMember& member : data.members)
			if (member.online)
				members.push_back(&member);

		guild_sort::sort(members, section.order, true);

		int16_t count = static_cast<int16_t>(members.size());

		if (section.guild_id != data.id)
		{
			section.guild_id = data.id;
			section.page = 1;
		}

		section.top = y;
		section.mark = GuildMark(data.emblem);
		section.marked = data.emblem.mark != 0;
		section.name.change_text(data.name);
		section.count.change_text(fraction(members.size(), data.members.size()));
		section.pages = count > 1 ? (count - 1) / UNION_PER_PAGE + 1 : 1;
		section.page = std::clamp<int16_t>(section.page, 1, section.pages);
		section.page_text.change_text(fraction(section.page, section.pages));

		y += GUILD_SECTION_HEADER;

		if (!section.expanded)
			return y + GUILD_SECTION_GAP;

		y += GUILD_COLUMNS_GAP + COLUMNS;
		section.first = y;

		int16_t last = std::min<int16_t>(count, section.page * UNION_PER_PAGE);

		for (int16_t i = (section.page - 1) * UNION_PER_PAGE; i < last; i++)
		{
			const GuildMember& member = *members[i];
			Color::Name color = member.id == alliance_picked ? Color::Name::WHITE : Color::Name::BLACK;
			std::string full_job = member.job < 0 ? "" : Job(static_cast<uint16_t>(member.job)).get_name();
			std::string job = member.job < 0 ? "??" : fit(Text::Font::A12M, full_job, MEMBER_JOB_WIDTH, "..");
			std::string level = member.level < 0 ? "??" : std::to_string(member.level);
			std::string title = alliance.title(member.alliance_rank);

			section.rows.push_back({
					member.id,
					y,
					Text(Text::Font::A12M, Text::Alignment::LEFT, color, member.name),
					Text(Text::Font::A12M, Text::Alignment::LEFT, color, job),
					Text(Text::Font::A12M, Text::Alignment::LEFT, color, level),
					Text(Text::Font::A12M, Text::Alignment::LEFT, color, title),
					job == full_job ? "" : full_job,
					member.level < 0,
					title
			});

			y += ROW;
		}

		if (count == 0)
			y += ROW;

		section.end = y;

		return y + GUILD_END + GUILD_SECTION_GAP;
	}

	void UIUserList::draw_alliance() const
	{
		Point<int16_t> header = position + GUILD_HEADER;

		// Without anyone online, as without an alliance, only the line that
		// says there is none (0x912b49).
		if (alliance_online > 0)
		{
			int16_t x = (GUILD_HEADER_WIDTH - alliance_name.width() - EMBLEM_SPACE) / 2 + EMBLEM_SPACE;
			alliance_name.draw(header + Point<int16_t>(x, UNION_NAME_Y + CLIENT_TEXT));
		}
		else
		{
			int16_t x = (GUILD_HEADER_WIDTH - no_alliance.width()) / 2;
			no_alliance.draw(header + Point<int16_t>(x, UNION_NAME_Y + CLIENT_TEXT));
		}

		union_notice_box.draw(position + NOTICE_BOX);

		if (alliance_online > 0)
		{
			if (shown_alliance.notice.empty())
			{
				union_notice_label.draw(position + NOTICE_LABEL + Point<int16_t>(0, CLIENT_TEXT));
			}
			else
			{
				Point<int16_t> strip = position + TICKER;
				union_notice.draw_between(strip + Point<int16_t>(union_ticker_x, CLIENT_TEXT),
										  Range<int16_t>(strip.x(), strip.x() + TICKER_WIDTH));
			}
		}

		int16_t view_top = alliance_scroll * GUILD_STEP;
		int16_t view_bottom = view_top + GUILD_VIEW;
		Point<int16_t> list = position + GUILD_LIST - Point<int16_t>(0, view_top);
		Range<int16_t> screen_rows = Range<int16_t>(position.y() + GUILD_LIST.y(),
													position.y() + GUILD_LIST.y() + GUILD_VIEW);

		auto art = [&](const Texture& texture, int16_t x, int16_t y)
		{
			int16_t from = std::max<int16_t>(0, view_top - y);
			int16_t to = std::min<int16_t>(texture.height(), view_bottom - y);

			if (from < to)
				texture.draw(list + Point<int16_t>(x, y), Rectangle<int16_t>(0, texture.width(), from, to));
		};

		auto words = [&](const Text& text, int16_t x, int16_t y)
		{
			if (y + text.height() > view_top && y < view_bottom)
				text.draw(list + Point<int16_t>(x, y + CLIENT_TEXT), screen_rows);
		};

		for (const AllianceSection& section : alliance_sections)
		{
			if (!section.shown)
				continue;

			art(union_header, 0, section.top);

			// A mark can't be cut, so it shows only whole.
			if (!section.marked)
				art(guild_frame, UNION_FRAME.x(), section.top + UNION_FRAME.y());
			else if (section.top + UNION_MARK.y() >= view_top &&
					 section.top + UNION_MARK.y() + GuildMark::SIZE <= view_bottom)
				section.mark.draw(list + Point<int16_t>(UNION_MARK.x(), section.top + UNION_MARK.y()));

			words(section.name, UNION_GUILD_NAME.x(), section.top + UNION_GUILD_NAME.y());
			words(section.count, UNION_COUNT.x(), section.top + UNION_COUNT.y());
			words(section.page_text, UNION_PAGE.x(), section.top + UNION_PAGE.y());

			if (!section.expanded)
				continue;

			art(union_columns, 0, section.first - COLUMNS);

			if (guild_sort::Column column = section.order.column)
			{
				int16_t top = section.first - COLUMNS + guild_sort::TOP;
				int16_t from = std::max<int16_t>(0, view_top - top);
				int16_t to = std::min<int16_t>(guild_sort::BOTTOM - guild_sort::TOP, view_bottom - top);
				int16_t left = guild_sort::EDGES[column - 1];

				if (from < to)
					union_sorted.draw(list + Point<int16_t>(1, top),
									  Rectangle<int16_t>(left - 1, guild_sort::EDGES[column] - 1, from, to));
			}

			if (section.rows.empty())
			{
				art(union_row, 0, section.first);
				words(none, MEMBER_NAME_X, section.first + MEMBER_TEXT_Y);
			}

			for (const GuildRow& row : section.rows)
			{
				art(union_row, 0, row.y);

				if (row.id == alliance_picked)
				{
					int16_t top = std::max(row.y, view_top);
					int16_t bottom = std::min<int16_t>(row.y + ROW, view_bottom);

					if (top < bottom)
						GraphicsGL::get().drawrectangle(list.x() + PICKED_LEFT, list.y() + top, PICKED_WIDTH,
														bottom - top, PICKED_RED, PICKED_GREEN, PICKED_BLUE, 1.0f);
				}

				words(row.name, MEMBER_NAME_X, row.y + MEMBER_TEXT_Y);
				words(row.job, MEMBER_JOB_X, row.y + MEMBER_TEXT_Y);
				words(row.level, MEMBER_LEVEL_X, row.y + MEMBER_TEXT_Y);
				words(row.title, MEMBER_TITLE_X, row.y + MEMBER_TEXT_Y);
			}

			art(union_end, 0, section.end);
		}

		alliance_slider.draw(position);
	}

	void UIUserList::place_alliance_buttons()
	{
		int16_t view_top = alliance_scroll * GUILD_STEP;
		Point<int16_t> list = GUILD_LIST - Point<int16_t>(0, view_top);

		for (uint16_t s = 0; s < UNION_SECTIONS; s++)
		{
			const AllianceSection& section = alliance_sections[s];
			uint16_t first = Buttons::BT_ALLIANCE_SECTION + s * SectionButton::SECTION_BUTTONS;
			// Made while the header's test point (77, y + 8) is in view
			// (FUN_0091ebd4).
			int16_t test_y = section.top + UNION_FOLD.y();
			bool shown = tab == Tab::ALLIANCE && section.shown && test_y >= view_top &&
						 test_y < view_top + GUILD_VIEW;
			Point<int16_t> top = list + Point<int16_t>(0, section.top);
			int16_t page_left = UNION_PAGE.x() - section.page_text.width() - PAGE_BUTTON;

			buttons[first + SectionButton::FOLD]->set_position(top + UNION_FOLD);
			buttons[first + SectionButton::UNFOLD]->set_position(top + UNION_FOLD);
			buttons[first + SectionButton::PREVIOUS_PAGE]->set_position(top + Point<int16_t>(page_left, UNION_PAGE.y()));
			buttons[first + SectionButton::NEXT_PAGE]->set_position(top + UNION_PAGE);

			buttons[first + SectionButton::FOLD]->set_active(shown && section.expanded);
			buttons[first + SectionButton::UNFOLD]->set_active(shown && !section.expanded);
			buttons[first + SectionButton::PREVIOUS_PAGE]->set_active(shown);
			buttons[first + SectionButton::NEXT_PAGE]->set_active(shown);

			enable(first + SectionButton::PREVIOUS_PAGE, section.page > 1);
			enable(first + SectionButton::NEXT_PAGE, section.page < section.pages);
		}
	}

	void UIUserList::scroll_alliance(int16_t scroll)
	{
		// As the guild tab's slider (FUN_0091e1eb).
		int16_t overflow = alliance_height - GUILD_VIEW;
		int16_t last = overflow > 0 ? overflow / GUILD_STEP + 1 : 0;

		alliance_scroll = std::clamp<int16_t>(scroll, 0, last);
		alliance_slider.setrows(alliance_scroll, 1, last + 1);
		place_alliance_buttons();
	}

	const GuildMember* UIUserList::picked_ally() const
	{
		if (alliance_picked == 0 || alliance_picked == Stage::get().get_player().get_oid())
			return nullptr;

		// The pick outlives a page change; it is a character, not a row.
		return alliance.find_member(guild, alliance_picked);
	}

	std::string UIUserList::alliance_tip(Point<int16_t> cursor_relative) const
	{
		if (over_notice(cursor_relative))
			return union_notice.get_text();

		if (cursor_relative.y() < TIP_LIST_TOP || cursor_relative.y() > TIP_LIST_BOTTOM)
			return "";

		int16_t x = cursor_relative.x() - GUILD_LIST.x();
		int16_t y = cursor_relative.y() - GUILD_LIST.y() + alliance_scroll * GUILD_STEP;

		// Only the first two guilds' counts and headings have tips.
		for (size_t i = 0; i < 2; i++)
		{
			const AllianceSection& section = alliance_sections[i];

			if (!section.shown)
				continue;

			int16_t right = TIP_UNION_COUNT_LEFT + TIP_COUNT_EXTRA + section.count.width();

			if (x >= TIP_UNION_COUNT_LEFT && x < right && y >= section.top + TIP_UNION_COUNT_TOP &&
				y < section.top + TIP_UNION_COUNT_BOTTOM)
				return TIP_ONLINE;
		}

		for (size_t i = 0; i < 2; i++)
		{
			const AllianceSection& section = alliance_sections[i];

			if (section.shown && section.expanded)
				if (guild_sort::Column column = guild_sort::column_at(x, y - (section.first - COLUMNS)))
					return TIP_SORT[column - 1];
		}

		if (x < 0 || x >= GUILD_ROW_WIDTH)
			return "";

		for (const AllianceSection& section : alliance_sections)
			for (const GuildRow& row : section.rows)
				if (y >= row.y && y < row.y + ROW)
					return row_tip(row, cursor_relative, x);

		return "";
	}

	Cursor::State UIUserList::alliance_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		Point<int16_t> cursor_relative = cursorpos - position;

		if (union_slide.alive && clicked && cursor_relative.x() >= dimension.x())
		{
			// Anyone may pick a rank (0x9181b2); a miss keeps the pick.
			Point<int16_t> in_panel = cursor_relative - union_panel();
			int16_t x = in_panel.x() - GRADE_ROW_LEFT;
			int16_t y = in_panel.y() - UNION_ROWS;
			int16_t rows = static_cast<int16_t>(union_titles.size());

			if (x >= 0 && x < GRADE_ROW_WIDTH && y >= 0 && y < ROW * rows)
			{
				int32_t rank = y / ROW + 1;

				if (rank != union_picked)
				{
					union_picked = rank;
					rebuild_union();
				}

				return Cursor::State::IDLE;
			}
		}

		std::string tip = alliance_tip(cursor_relative);
		show_tip(tip.empty() ? nullptr : tip.c_str());

		if (Cursor::State state = alliance_slider.send_cursor(cursor_relative, clicked))
			return state;

		if (!clicked || !UIElement::is_in_range(cursorpos))
			return UIElement::send_cursor(clicked, cursorpos);

		for (auto& [id, button] : buttons)
			if (button->is_active() && button->bounds(position).contains(cursorpos))
				return UIElement::send_cursor(clicked, cursorpos);

		// Any other click on the window picks the row under it, or no one
		// (0x915534). The list is hit as if it ran on under the header.
		int16_t x = cursor_relative.x() - GUILD_LIST.x();
		int16_t y = cursor_relative.y() - GUILD_LIST.y() + alliance_scroll * GUILD_STEP;
		int32_t picked = 0;

		if (x >= 0 && x < GUILD_ROW_WIDTH)
			for (const AllianceSection& section : alliance_sections)
				for (const GuildRow& row : section.rows)
					if (y >= row.y && y < row.y + ROW)
						picked = row.id;

		// Then a click on a section's headings sorts it (0x915456).
		bool sorted = false;

		for (AllianceSection& section : alliance_sections)
		{
			if (!section.shown || !section.expanded)
				continue;

			if (guild_sort::Column column = guild_sort::column_at(x, y - (section.first - COLUMNS)))
			{
				section.order.click(column);
				sorted = true;
				break;
			}
		}

		if (picked != alliance_picked || sorted)
		{
			alliance_picked = picked;
			rebuild_alliance();
			update_buttons();
		}

		return Cursor::State::IDLE;
	}

	void UIUserList::update_alliance_buttons(bool party_leader)
	{
		int32_t self = Stage::get().get_player().get_oid();
		const GuildMember* picked = picked_ally();
		Alliance::TabButtons rules = alliance.tab_buttons(guild, self, picked ? picked->id : 0, party_leader);

		enable(Buttons::BT_ALLIANCE_INVITE, rules.invite);
		enable(Buttons::BT_ALLIANCE_WITHDRAW, rules.withdraw);
		enable(Buttons::BT_ALLIANCE_KICK, rules.kick);
		enable(Buttons::BT_ALLIANCE_UP, rules.up);
		enable(Buttons::BT_ALLIANCE_DOWN, rules.down);
		enable(Buttons::BT_ALLIANCE_CHANGE, rules.change);
		enable(Buttons::BT_ALLIANCE_CHAT, rules.chat);
		enable(Buttons::BT_ALLIANCE_INFO, rules.info);
		enable(Buttons::BT_ALLIANCE_WHISPER, rules.whisper);
		enable(Buttons::BT_ALLIANCE_NOTICE, rules.notice);
		enable(Buttons::BT_ALLIANCE_PARTY_INVITE, rules.party_invite);

		bool master = alliance.may_edit_titles(guild, self);
		enable(Buttons::BT_UNION_MODIFY, master);
		enable(Buttons::BT_UNION_SAVE, master);
	}

	Button::State UIUserList::alliance_button(uint16_t buttonid)
	{
		const GuildMember* picked = picked_ally();
		int32_t self = Stage::get().get_player().get_oid();

		switch (buttonid)
		{
			case Buttons::BT_ALLIANCE_INVITE:
				prompt_union_invite();
				break;
			case Buttons::BT_ALLIANCE_WITHDRAW:
				UI::get().emplace<UIYesNo>(
						"Are you sure you want to leave the Union?",
						[](bool yes)
						{
							if (yes)
								LeaveAlliancePacket().dispatch();
						}
				);
				break;
			case Buttons::BT_ALLIANCE_KICK:
				if (picked)
				{
					if (const GuildData* theirs = alliance.guild_of(guild, picked->id))
					{
						int32_t guild_id = theirs->id;
						int32_t alliance_id = guild.get_data().alliance_id;

						UI::get().emplace<UIYesNo>(
								"Do you wish to remove [" + theirs->name + "] Guild\\nfrom the Union?",
								[guild_id, alliance_id](bool yes)
								{
									if (yes)
										ExpelFromAlliancePacket(guild_id, alliance_id).dispatch();
								}
						);
					}
				}

				break;
			// Without asking (0x91505b, 0x915983).
			case Buttons::BT_ALLIANCE_UP:
			case Buttons::BT_ALLIANCE_DOWN:
				if (picked)
					ChangeAllianceRankPacket(picked->id, buttonid == Buttons::BT_ALLIANCE_UP).dispatch();

				break;
			case Buttons::BT_ALLIANCE_CHANGE:
				if (picked)
					ChangeAllianceMasterPacket(picked->id).dispatch();

				break;
			// A messenger, as the Friend tab's Chat (0x91517b).
			case Buttons::BT_ALLIANCE_CHAT:
				if (picked)
				{
					auto messenger = UI::get().get_element<UIMessenger>();

					if (!messenger || !messenger->is_active())
					{
						MessengerOpenPacket(0).dispatch();
						MessengerInvitePacket(picked->name).dispatch();
					}
				}

				break;
			case Buttons::BT_ALLIANCE_INFO:
				if (union_slide.out)
					close_union();
				else
					open_union();

				break;
			case Buttons::BT_ALLIANCE_WHISPER:
				if (picked)
					if (auto chatbar = UI::get().get_element<UIChatBar>())
						chatbar->whisper_to(picked->name);

				break;
			// Cancel or a blank notice sends nothing.
			case Buttons::BT_ALLIANCE_NOTICE:
				UI::get().emplace<UIEnterText>(
						"Enter the Guild Union notice.",
						[](const std::string& text)
						{
							std::string notice = trim(text);

							if (!notice.empty())
								SetAllianceNoticePacket(CurseData::filter(notice)).dispatch();
						},
						NOTICE_LIMIT,
						true
				);
				break;
			case Buttons::BT_ALLIANCE_PARTY_INVITE:
				if (picked)
					PartyCommands::invite(picked->name);

				break;
			case Buttons::BT_UNION_MODIFY:
				if (union_picked > 0 && alliance.may_edit_titles(guild, self))
					prompt_union_title(union_picked);

				break;
			case Buttons::BT_UNION_SAVE:
				if (alliance.may_edit_titles(guild, self))
				{
					UI::get().emplace<UIYesNo>(
							"Would like to save the change?",
							[](bool yes)
							{
								if (yes)
									if (auto userlist = UI::get().get_element<UIUserList>())
										userlist->save_union_titles();
							}
					);
				}

				break;
			default:
				if (buttonid >= Buttons::BT_ALLIANCE_SECTION && buttonid < Buttons::BT_TAB0)
				{
					uint16_t index = buttonid - Buttons::BT_ALLIANCE_SECTION;
					AllianceSection& section = alliance_sections[index / SectionButton::SECTION_BUTTONS];

					switch (index % SectionButton::SECTION_BUTTONS)
					{
						case SectionButton::FOLD:
						case SectionButton::UNFOLD:
							section.expanded = !section.expanded;
							break;
						case SectionButton::PREVIOUS_PAGE:
							section.page--;
							break;
						case SectionButton::NEXT_PAGE:
							section.page++;
							break;
					}

					rebuild_alliance();

					if (buttons[buttonid]->get_state() == Button::State::DISABLED)
						return Button::State::DISABLED;
				}

				break;
		}

		return Button::State::NORMAL;
	}

	// Each opening builds the panel anew from the alliance's titles, even
	// one still sliding back (FUN_0091e66f).
	void UIUserList::open_union()
	{
		union_slide.open(false);
		union_titles = alliance.get_data().titles;
		union_picked = 0;

		place_panel_buttons();
		rebuild_union();
		update_buttons();
	}

	void UIUserList::close_union()
	{
		union_slide.close();
	}

	Point<int16_t> UIUserList::union_panel() const
	{
		return UNION + Point<int16_t>(union_slide.x - RankPanelSlide::TRAVEL, 0);
	}

	void UIUserList::rebuild_union()
	{
		for (size_t i = 0; i < union_titles.size(); i++)
		{
			Color::Name color = union_picked == static_cast<int32_t>(i) + 1 ? Color::Name::WHITE : Color::Name::BLACK;
			Color::Name label = union_titles[i].empty() ? Color::Name::GRADEGREY : color;

			union_ranks[i] = Text(Text::Font::A12M, Text::Alignment::LEFT, color, std::to_string(i + 1));
			union_title_texts[i] = Text(Text::Font::A12M, Text::Alignment::LEFT, color, union_titles[i]);
			union_labels[i] = Text(Text::Font::A12M, Text::Alignment::LEFT, label, GRADE_LABELS[i]);
		}
	}

	void UIUserList::draw_union() const
	{
		Point<int16_t> panel = position + union_panel();

		union_background.draw(panel);

		if (!alliance.joined(guild))
			return;

		for (size_t i = 0; i < union_art.size(); i++)
			union_art[i].draw(panel + UNION_ART[i].second);

		for (int16_t i = 0; i < static_cast<int16_t>(union_titles.size()); i++)
		{
			int16_t top = UNION_ROWS + ROW * i;
			int16_t y = top + GRADE_TEXT_Y + CLIENT_TEXT;

			if (union_picked == i + 1)
				GraphicsGL::get().drawrectangle(panel.x() + GRADE_ROW_LEFT, panel.y() + top, GRADE_ROW_WIDTH, ROW,
												PICKED_RED, PICKED_GREEN, PICKED_BLUE, 1.0f);

			const Text& label = union_labels[i];

			union_ranks[i].draw(panel + Point<int16_t>(GRADE_RANK_X, y));
			union_title_texts[i].draw(panel + Point<int16_t>(GRADE_TITLE_X, y));
			label.draw(panel + Point<int16_t>(GRADE_LABEL_X + (GRADE_LABEL_WIDTH - label.width()) / 2, y));
		}
	}

	void UIUserList::prompt_union_title(int32_t rank)
	{
		UI::get().emplace<UIEnterText>(
				"Please put a position,\\n which you want to change to.",
				[rank](const std::string& text)
				{
					if (auto userlist = UI::get().get_element<UIUserList>())
						userlist->edit_union_title(rank, text);
				},
				Guild::MAX_TITLE,
				true,
				Guild::min_title(rank)
		);
	}

	void UIUserList::edit_union_title(int32_t rank, const std::string& text)
	{
		std::string title = trim(text);
		Guild::Title check = alliance.check_title(guild, union_titles, rank, title);

		if (check == Guild::Title::OK && !title.empty() && CurseData::in_name(title))
			check = Guild::Title::NOT_ALLOWED;

		switch (check)
		{
			case Guild::Title::OK:
				Guild::set_title(union_titles, rank, title);
				rebuild_union();
				break;
			case Guild::Title::TOO_SHORT:
				UI::get().emplace<UIOk>(
						"Please enter at least " + std::to_string(Guild::min_title(rank)) + " characters.",
						[rank](bool)
						{
							if (auto userlist = UI::get().get_element<UIUserList>())
								userlist->prompt_union_title(rank);
						}
				);
				break;
			case Guild::Title::NOT_ALLOWED:
				UI::get().emplace<UIOk>("It cannot be used. Please check it again.", [](bool) {});
				break;
			case Guild::Title::LOWER_RANK:
				UI::get().emplace<UIOk>(
						"You cannot delete this position, because\\nthere is a lower rank than this position.",
						[](bool) {}
				);
				break;
			case Guild::Title::HELD:
				UI::get().emplace<UIOk>(
						"Unable to delete due to a member\\nin the Union currently being on the position.",
						[](bool) {}
				);
				break;
		}
	}

	// Unchanged titles only say so (0x91848d); the server's answer says the
	// rest.
	void UIUserList::save_union_titles()
	{
		if (alliance.titles_differ(union_titles))
			SetAllianceTitlesPacket(union_titles).dispatch();
		else
			UI::get().emplace<UIOk>("It is saved.", [](bool) {});
	}
}
