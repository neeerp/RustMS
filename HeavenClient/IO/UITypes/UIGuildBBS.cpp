#include "UIGuildBBS.h"

#include "UINotice.h"

#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Constants.h"
#include "../../Gameplay/Stage.h"
#include "../../Net/Packets/GameplayPackets.h"
#include "../../Util/Misc.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

#include <algorithm>
#include <ctime>
#include <string>

namespace ms
{
	namespace
	{
		// v83 places text about 3 px lower than ours.
		constexpr int16_t TEXT_RAISE = 3;

		const Point<int16_t> EMBLEM_AT(20, 24), GUILD_NAME_AT(58, 22);
		const Point<int16_t> VIEW_PANEL_AT(399, 18);

		// The list's columns (0x8159d8), from a row's top.
		constexpr int16_t TEXT_X = 26, COUNT_GAP = 2, ICON_X = 237, WRITER_X = 257, WRITER_WIDTH = 66,
						  WRITER_MAX = 70, DATE_X = 332, TEXT_Y = 8, ICON_Y = 5;

		// The page selector (control 0x7e5): 320 px from (40, 450), ten
		// labels to a group. The original's pitch is unknown; this spreads a
		// group over the width.
		constexpr int16_t PAGES_X = 40, PAGES_Y = 450, PAGES_WIDTH = 320, PAGE_PITCH = PAGES_WIDTH / 10,
						  PAGE_HEIGHT = 16;

		// The thread view (0x814c2c).
		constexpr int16_t TITLE_X = 409, TITLE_WIDTH = 300, TITLE_Y = 36;
		const Point<int16_t> POSTER_AT(445, 63), POSTED_AT(576, 63);
		// The read-only body: 250 x 180 at (424, 83), wrapping at 240 px.
		// Lines are 12 px apart; our font's fifteenth would cross the
		// panel's edge, so 14 show.
		const Point<int16_t> BODY_AT(424, 83);
		constexpr int16_t BODY_WRAP = 240, BODY_HEIGHT = 180, BODY_LINE = 12, BODY_LINES = 14;
		constexpr int16_t BODY_SLIDER_X = 704, BODY_SLIDER_TOP = 78, BODY_SLIDER_BOTTOM = 268;
		// Four replies from y 335, 31 px apart: the writer centred in 76 px
		// from x 399, the text at 480, the date at 654.
		constexpr int16_t REPLIES_Y = 335, REPLY_HEIGHT = 31, REPLY_WRITER_X = 399, REPLY_WRITER_WIDTH = 76,
						  REPLY_TEXT_X = 480, REPLY_DATE_X = 654;
		constexpr int16_t REPLY_SLIDER_X = 710, REPLY_SLIDER_TOP = 326, REPLY_SLIDER_BOTTOM = 451;

		// Buttons, by top-left (control table 0x813a8c).
		const Point<int16_t> NOTICE_AT(404, 275), WRITE_AT(540, 275), RETOUCH_AT(602, 275), DELETE_AT(664, 275),
							 REPLY_AT(704, 457), REPLY_DELETE_AT(690, 331), QUIT_AT(680, 501);

		// 0xe8a, 0xe9f
		constexpr const char* DELETE_QUESTION = "Are you sure you want to delete the post?";
		constexpr const char* THREAD_GONE = "This message had been deleted by another user.\\nPlease check again.";

		Text plain_text(Text::Font font, Color::Name color, const std::string& str)
		{
			return Text(font, Text::Alignment::LEFT, color, str, 0, false);
		}

		item_expiry::Time today()
		{
			std::time_t now = std::time(nullptr);
			std::tm local = *std::localtime(&now);

			return { local.tm_year + 1900, local.tm_mon + 1, local.tm_mday, local.tm_hour, local.tm_min };
		}

		// A member's name, or "" for someone no longer in the guild
		// (0xa391b3).
		std::string member_name(int32_t cid)
		{
			const GuildMember* member = Stage::get().get_player().get_guild().find(cid);

			return member ? member->name : "";
		}

		int32_t own_rank()
		{
			const Player& player = Stage::get().get_player();

			return player.get_guild().rank(player.get_oid());
		}

		int32_t own_id()
		{
			return Stage::get().get_player().get_oid();
		}
	}

	void UIGuildBBS::open()
	{
		auto board = UI::get().get_element<UIGuildBBS>();

		if (board && board->is_active())
			return;

		UI::get().emplace<UIGuildBBS>();
	}

	UIGuildBBS::UIGuildBBS() : UIElement(Point<int16_t>(), Point<int16_t>()), emblem_shown(false), page(0),
		body_scroll(0), reply_scroll(0), mouse_down(false), wheel_on_body(false)
	{
		nl::node src = NxFiles::ui_classic["GuildBBS.img"]["GuildBBS"];

		background = src["backgrnd"];
		view_panel = src["backgrnd2"];
		guild_frame = NxFiles::ui_classic["UIWindow.img"]["UserList"]["Guild"]["GuildInfo"]["guildmark"];

		for (size_t i = 0; i < basic_icons.size(); i++)
			basic_icons[i] = src["Emoticon"]["Basic"][std::to_string(i)];

		for (size_t i = 0; i < cash_icons.size(); i++)
			cash_icons[i] = src["Emoticon"]["Cash"][std::to_string(i)];

		auto button = [&](uint16_t id, const char* name, Point<int16_t> topleft)
		{
			nl::node node = src[name];
			buttons[id] = std::make_unique<MapleButton>(node, topleft + Texture(node["normal"]["0"]).get_origin());
		};

		button(BT_NOTICE, "BtNotice", NOTICE_AT);
		button(BT_WRITE, "BtWrite", WRITE_AT);
		button(BT_RETOUCH, "BtRetouch", RETOUCH_AT);
		button(BT_DELETE, "BtDelete", DELETE_AT);
		button(BT_REPLY, "BtReply", REPLY_AT);

		for (uint16_t k = 0; k < GuildBbs::REPLY_ROWS; k++)
			button(BT_REPLY_DELETE0 + k, "BtReplyDelete", REPLY_DELETE_AT + Point<int16_t>(0, REPLY_HEIGHT * k));

		button(BT_QUIT, "BtQuit", QUIT_AT);

		body_slider = Slider(Slider::Type::CLASSIC, Range<int16_t>(BODY_SLIDER_TOP, BODY_SLIDER_BOTTOM), BODY_SLIDER_X,
				BODY_LINES, 0, [&](bool up) { scroll_body(body_scroll + (up ? -1 : 1)); });
		reply_slider = Slider(Slider::Type::CLASSIC, Range<int16_t>(REPLY_SLIDER_TOP, REPLY_SLIDER_BOTTOM),
				REPLY_SLIDER_X, GuildBbs::REPLY_ROWS, 0, [&](bool up) { scroll_replies(reply_scroll + (up ? -1 : 1)); });

		dimension = background.get_dimensions();
		update_screen(Constants::Constants::get().get_viewwidth(), Constants::Constants::get().get_viewheight());

		build_header();
		build_pages();
		build_view();

		// The window opens empty and asks for the first page (0x816b69).
		BbsListPacket(page).dispatch();
	}

	void UIGuildBBS::update_screen(int16_t new_width, int16_t new_height)
	{
		position = Point<int16_t>((new_width - dimension.x()) / 2, (new_height - dimension.y()) / 2);
	}

	void UIGuildBBS::draw(float inter) const
	{
		Point<int16_t> raise(0, TEXT_RAISE);

		background.draw(position + background.get_origin());
		view_panel.draw(position + VIEW_PANEL_AT + view_panel.get_origin());

		if (emblem_shown)
			emblem.draw(position + EMBLEM_AT);
		else
			guild_frame.draw(position + EMBLEM_AT + guild_frame.get_origin());

		guild_name.draw(position + GUILD_NAME_AT - raise);

		if (notice_row)
			draw_row(*notice_row, GuildBbs::NOTICE_TOP);

		for (size_t i = 0; i < rows.size(); i++)
			draw_row(rows[i], GuildBbs::ROWS_TOP + GuildBbs::ROW_HEIGHT * static_cast<int16_t>(i));

		for (const PageLink& link : page_links)
			link.label.draw(position + Point<int16_t>(link.text_x, PAGES_Y) - raise);

		if (thread)
		{
			title.draw(position + Point<int16_t>(TITLE_X + (TITLE_WIDTH - title.width()) / 2, TITLE_Y) - raise);
			poster.draw(position + POSTER_AT - raise);
			posted.draw(position + POSTED_AT - raise);

			int16_t top = position.y() + BODY_AT.y();
			Range<int16_t> shown(top, top + BODY_HEIGHT);

			for (size_t i = body_scroll; i < body.size() && i < body_scroll + static_cast<size_t>(BODY_LINES); i++)
			{
				int16_t y = BODY_AT.y() + BODY_LINE * static_cast<int16_t>(i - body_scroll);
				body[i].draw(DrawArgument(position + Point<int16_t>(BODY_AT.x(), y) - raise), shown);
			}

			for (size_t k = 0; k < GuildBbs::REPLY_ROWS && reply_scroll + k < replies.size(); k++)
			{
				const ReplyRow& reply = replies[reply_scroll + k];
				int16_t y = REPLIES_Y + REPLY_HEIGHT * static_cast<int16_t>(k) - TEXT_RAISE;
				int16_t writer_x = REPLY_WRITER_X + (REPLY_WRITER_WIDTH - reply.writer.width()) / 2;

				reply.writer.draw(position + Point<int16_t>(writer_x, y));
				reply.text.draw(position + Point<int16_t>(REPLY_TEXT_X, y));
				reply.date.draw(position + Point<int16_t>(REPLY_DATE_X, y));
			}
		}

		body_slider.draw(position);
		reply_slider.draw(position);

		UIElement::draw(inter);
	}

	UIGuildBBS::Row UIGuildBBS::make_row(const GuildBbs::Entry& entry) const
	{
		Row row;
		row.text = plain_text(Text::Font::A12M, Color::Name::BLACK, GuildBbs::row_text(entry.id, entry.title));
		row.count = plain_text(Text::Font::A12M, Color::Name::PICTONBLUE, GuildBbs::count_text(entry.replies));

		if (entry.icon >= 0 && entry.icon < static_cast<int32_t>(basic_icons.size()))
			row.icon = basic_icons[entry.icon];
		else if (entry.icon >= 100 && entry.icon < 100 + static_cast<int32_t>(cash_icons.size()))
			row.icon = cash_icons[entry.icon - 100];

		row.writer = plain_text(Text::Font::A12M, Color::Name::BLACK, member_name(entry.poster));
		string_format::format_with_ellipsis(row.writer, WRITER_MAX);
		row.date = plain_text(Text::Font::A12M, Color::Name::DARKGREY, GuildBbs::short_date(entry.time, today()));

		return row;
	}

	void UIGuildBBS::draw_row(const Row& row, int16_t y) const
	{
		Point<int16_t> top = position + Point<int16_t>(0, y);
		int16_t text_y = TEXT_Y - TEXT_RAISE;

		row.text.draw(top + Point<int16_t>(TEXT_X, text_y));
		row.count.draw(top + Point<int16_t>(TEXT_X + COUNT_GAP + row.text.width(), text_y));
		row.icon.draw(top + Point<int16_t>(ICON_X, ICON_Y) + row.icon.get_origin());
		row.writer.draw(top + Point<int16_t>(WRITER_X + (WRITER_WIDTH - row.writer.width()) / 2, text_y));
		row.date.draw(top + Point<int16_t>(DATE_X, text_y));
	}

	void UIGuildBBS::build_header()
	{
		const GuildData& data = Stage::get().get_player().get_guild().get_data();

		// The frame unless there is a mark, as on the Guild tab.
		emblem_shown = data.emblem.mark != 0;
		emblem = GuildMark(data.emblem);
		guild_name = plain_text(Text::Font::A12M, Color::Name::BLACK, data.name);
	}

	void UIGuildBBS::build_pages()
	{
		page_links.clear();

		int32_t pages = GuildBbs::pages(list.total);
		int32_t first = std::clamp(page, 0, pages - 1) / 10 * 10;
		int32_t last = std::min(first + 10, pages);

		for (int32_t p = first; p < last; p++)
		{
			Color::Name color = p == page ? Color::Name::PICTONBLUE : Color::Name::BLACK;
			Text label(Text::Font::A12M, Text::Alignment::LEFT, color, std::to_string(p + 1));
			int16_t left = PAGES_X + PAGE_PITCH * static_cast<int16_t>(p - first);

			page_links.push_back({ p, label, left, static_cast<int16_t>(left + (PAGE_PITCH - label.width()) / 2),
					PAGE_PITCH });
		}

		// Other groups of ten: the original's art for these is unknown.
		if (first > 0)
		{
			Text label(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, "<");
			int16_t left = PAGES_X - 2 - label.width();
			page_links.insert(page_links.begin(), { first - 1, label, left, left, label.width() });
		}

		if (last < pages)
		{
			Text label(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, ">");
			int16_t left = PAGES_X + PAGES_WIDTH + 2;
			page_links.push_back({ last, label, left, left, label.width() });
		}
	}

	void UIGuildBBS::build_view()
	{
		int32_t rank = own_rank();
		bool editable = thread && GuildBbs::may_edit(rank, thread->poster, own_id());

		// Not built yet: writing, the notice, editing, and replying.
		buttons[BT_NOTICE]->set_state(Button::State::DISABLED);
		buttons[BT_WRITE]->set_state(Button::State::DISABLED);
		buttons[BT_RETOUCH]->set_state(Button::State::DISABLED);
		buttons[BT_REPLY]->set_state(Button::State::DISABLED);
		buttons[BT_DELETE]->set_state(editable ? Button::State::NORMAL : Button::State::DISABLED);

		title = Text();
		poster = Text();
		posted = Text();
		body.clear();
		replies.clear();
		body_scroll = 0;
		reply_scroll = 0;

		if (thread)
		{
			title = plain_text(Text::Font::A12B, Color::Name::BLACK, thread->title);
			poster = plain_text(Text::Font::A12M, Color::Name::DARKGREY, member_name(thread->poster));
			posted = plain_text(Text::Font::A12M, Color::Name::DARKGREY, GuildBbs::full_date(thread->time));

			auto width = [](const std::string& line)
			{
				return Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, line, 0, false).width();
			};

			for (const std::string& line : GuildBbs::body_lines(thread->body, BODY_WRAP, width))
				body.push_back(plain_text(Text::Font::A12M, Color::Name::BLACK, line));

			item_expiry::Time now = today();

			for (const GuildBbs::Reply& reply : thread->replies)
			{
				replies.push_back({ plain_text(Text::Font::A12M, Color::Name::DARKGREY, member_name(reply.poster)),
									plain_text(Text::Font::A12M, Color::Name::DARKGREY, reply.text),
									plain_text(Text::Font::A12M, Color::Name::DARKGREY, GuildBbs::short_date(reply.time, now)) });
			}

			reply_scroll = static_cast<int16_t>(GuildBbs::reply_start(replies.size()));
		}

		body_slider.setrows(0, BODY_LINES, static_cast<int16_t>(body.size()));
		reply_slider.setrows(reply_scroll, GuildBbs::REPLY_ROWS, static_cast<int16_t>(replies.size()));

		show_reply_buttons();
	}

	void UIGuildBBS::show_reply_buttons()
	{
		int32_t rank = own_rank();
		int32_t self = own_id();

		for (size_t k = 0; k < GuildBbs::REPLY_ROWS; k++)
		{
			size_t index = reply_scroll + k;
			bool shown = thread && index < thread->replies.size()
				&& GuildBbs::may_edit(rank, thread->replies[index].poster, self);

			buttons[BT_REPLY_DELETE0 + k]->set_active(shown);
		}
	}

	void UIGuildBBS::scroll_body(int16_t to)
	{
		int16_t last = std::max<int16_t>(0, static_cast<int16_t>(body.size()) - BODY_LINES);
		body_scroll = std::clamp<int16_t>(to, 0, last);
	}

	void UIGuildBBS::scroll_replies(int16_t to)
	{
		int16_t last = static_cast<int16_t>(GuildBbs::reply_start(replies.size()));
		reply_scroll = std::clamp<int16_t>(to, 0, last);
		show_reply_buttons();
	}

	void UIGuildBBS::show_list(const GuildBbs::List& received)
	{
		list = received;
		// The server sends the first page for one past the last, as after
		// deleting the last page's only thread.
		page = GuildBbs::served_page(page, list.total);
		notice_row.reset();
		rows.clear();

		if (list.has_notice)
			notice_row = make_row(list.notice);

		for (const GuildBbs::Entry& entry : list.entries)
			rows.push_back(make_row(entry));

		build_header();
		build_pages();
	}

	void UIGuildBBS::show_thread(GuildBbs::Thread received)
	{
		thread = std::move(received);
		build_view();
	}

	void UIGuildBBS::thread_missing()
	{
		UI::get().emplace<UIOk>(THREAD_GONE, [](bool) {});

		thread.reset();
		build_view();
	}

	void UIGuildBBS::remove_cursor()
	{
		UIElement::remove_cursor();

		body_slider.remove_cursor();
		reply_slider.remove_cursor();
		mouse_down = false;
	}

	Cursor::State UIGuildBBS::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		bool pressed = clicked && !mouse_down;
		mouse_down = clicked;

		Point<int16_t> local = cursorpos - position;
		// The wheel scrolls the body above the buttons and the replies
		// anywhere else.
		wheel_on_body = local.x() >= VIEW_PANEL_AT.x() && local.y() < NOTICE_AT.y();

		if (Cursor::State state = body_slider.send_cursor(local, clicked))
			return state;

		if (Cursor::State state = reply_slider.send_cursor(local, clicked))
			return state;

		if (pressed)
		{
			int32_t row = GuildBbs::row_at(local.x(), local.y(), list.has_notice, list.entries.size());

			if (row != GuildBbs::NO_ROW)
			{
				// The notice is thread 0 (0x81621c).
				int32_t id = row == GuildBbs::NOTICE_ROW ? 0 : list.entries[row].id;

				BbsShowPacket(id).dispatch();
				BbsListPacket(page).dispatch();

				return Cursor::State::IDLE;
			}

			for (const PageLink& link : page_links)
			{
				if (local.x() >= link.left && local.x() < link.left + link.width
					&& local.y() >= PAGES_Y - TEXT_RAISE && local.y() < PAGES_Y - TEXT_RAISE + PAGE_HEIGHT)
				{
					// The thread view stays.
					page = link.page;
					BbsListPacket(page).dispatch();

					return Cursor::State::IDLE;
				}
			}
		}

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UIGuildBBS::send_scroll(double yoffset)
	{
		if (wheel_on_body)
			body_slider.send_scroll(yoffset);
		else
			reply_slider.send_scroll(yoffset);
	}

	Button::State UIGuildBBS::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
		case BT_DELETE:
			ask_delete_thread();
			break;
		case BT_QUIT:
			deactivate();
			break;
		default:
			if (buttonid >= BT_REPLY_DELETE0 && buttonid <= BT_REPLY_DELETE3)
				ask_delete_reply(buttonid - BT_REPLY_DELETE0);

			break;
		}

		return Button::State::NORMAL;
	}

	void UIGuildBBS::ask_delete_thread()
	{
		if (!thread)
			return;

		int32_t id = thread->id;

		// Nothing answers the delete; the list request brings the change
		// (0x8168db).
		UI::get().emplace<UIYesNo>(DELETE_QUESTION, [id](bool yes)
		{
			auto board = UI::get().get_element<UIGuildBBS>();

			if (!yes || !board || !board->is_active())
				return;

			BbsDeleteThreadPacket(id).dispatch();

			board->thread.reset();
			board->build_view();

			BbsListPacket(board->page).dispatch();
		});
	}

	void UIGuildBBS::ask_delete_reply(size_t row)
	{
		size_t index = reply_scroll + row;

		if (!thread || index >= thread->replies.size())
			return;

		int32_t thread_id = thread->id;
		int32_t reply_id = thread->replies[index].id;

		// The server answers with the thread and the list (0x816aa8).
		UI::get().emplace<UIYesNo>(DELETE_QUESTION, [thread_id, reply_id](bool yes)
		{
			auto board = UI::get().get_element<UIGuildBBS>();

			if (!yes || !board || !board->is_active())
				return;

			BbsDeleteReplyPacket(thread_id, reply_id).dispatch();
			BbsListPacket(board->page).dispatch();
		});
	}

	UIElement::Type UIGuildBBS::get_type() const
	{
		return TYPE;
	}
}
