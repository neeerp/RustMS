#include "UIGuildBBS.h"

#include "UINotice.h"

#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Constants.h"
#include "../../Data/CurseData.h"
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

		// The reply input (0x7e2): white on the dark bar.
		const Rectangle<int16_t> REPLY_BOX(424, 680, 459 - TEXT_RAISE, 475 - TEXT_RAISE);

		// The write panel (0x8145a9): the title, the body (wrapping at
		// 240 px, 15 lines of 12 px), and the body's scroll bar.
		const Rectangle<int16_t> TITLE_BOX(449, 705, 30 - TEXT_RAISE, 46 - TEXT_RAISE);
		const Rectangle<int16_t> DRAFT_BOX(449, 699, 56 - TEXT_RAISE, 236 - TEXT_RAISE);
		constexpr int16_t DRAFT_SLIDER_X = 706, DRAFT_SLIDER_TOP = 53, DRAFT_SLIDER_BOTTOM = 240;
		const Point<int16_t> REGISTER_AT(602, 275), CANCEL_AT(664, 275), LEFT_AT(404, 243), RIGHT_AT(704, 243);
		// The palette's icons and the highlight's frame, by top-left (0x814c2c).
		const Point<int16_t> PALETTE_ICON_AT(427, 247), PALETTE_FRAME_AT(425, 245);

		// 0xe8a, 0xe9f, 0x15b9, 0xe89, 0x110, 0xe8b
		constexpr const char* DELETE_QUESTION = "Are you sure you want to delete the post?";
		constexpr const char* THREAD_GONE = "This message had been deleted by another user.\\nPlease check again.";
		constexpr const char* NO_TITLE = "Please enter the title.";
		constexpr const char* NO_TEXT = "Please enter the text.";
		constexpr const char* NO_CURSING = "Cursing is not allowed.";
		constexpr const char* NOTICE_TAKEN = "Please delete the current notice\\nand then try again.";

		// The text for a refusal, or nothing.
		const char* refusal_text(GuildBbs::Refusal refusal)
		{
			switch (refusal)
			{
			case GuildBbs::Refusal::NO_TITLE:
				return NO_TITLE;
			case GuildBbs::Refusal::NO_TEXT:
				return NO_TEXT;
			case GuildBbs::Refusal::CURSING:
				return NO_CURSING;
			default:
				return nullptr;
			}
		}

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
		selected(0), body_scroll(0), reply_scroll(0), highlight(0), icon(0), mouse_down(false), wheel_on_body(false)
	{
		nl::node src = NxFiles::ui_classic["GuildBBS.img"]["GuildBBS"];

		background = src["backgrnd"];
		view_panel = src["backgrnd2"];
		write_panel = src["backgrnd3"];
		select_frame = src["Emoticon"]["Select"];
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
		button(BT_REGISTER, "BtRegister", REGISTER_AT);
		button(BT_CANCEL, "BtCancel", CANCEL_AT);

		buttons[BT_LEFT] = std::make_unique<MapleButton>(src["MoveEmoticon"]["BtLeft"], LEFT_AT);
		buttons[BT_RIGHT] = std::make_unique<MapleButton>(src["MoveEmoticon"]["BtRight"], RIGHT_AT);

		body_slider = Slider(Slider::Type::CLASSIC, Range<int16_t>(BODY_SLIDER_TOP, BODY_SLIDER_BOTTOM), BODY_SLIDER_X,
				BODY_LINES, 0, [&](bool up) { scroll_body(body_scroll + (up ? -1 : 1)); });
		reply_slider = Slider(Slider::Type::CLASSIC, Range<int16_t>(REPLY_SLIDER_TOP, REPLY_SLIDER_BOTTOM),
				REPLY_SLIDER_X, GuildBbs::REPLY_ROWS, 0, [&](bool up) { scroll_replies(reply_scroll + (up ? -1 : 1)); });
		draft_slider = Slider(Slider::Type::CLASSIC, Range<int16_t>(DRAFT_SLIDER_TOP, DRAFT_SLIDER_BOTTOM),
				DRAFT_SLIDER_X, GuildBbs::BODY_ROWS, 0, [&](bool up)
				{
					size_t first = body_field.first_line();
					body_field.scroll_lines(up ? (first > 0 ? first - 1 : 0) : first + 1);
				});

		reply_field = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, REPLY_BOX,
				GuildBbs::REPLY_LIMIT);

		title_field = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, TITLE_BOX,
				GuildBbs::TITLE_LIMIT);
		body_field = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, DRAFT_BOX,
				GuildBbs::BODY_LIMIT);
		// Enter sends neither the reply nor the title, and Tab moves no focus
		// (0x816994, 0x8145a9).
		body_field.set_lines(BODY_WRAP, GuildBbs::BODY_ROWS, BODY_LINE);

		dimension = background.get_dimensions();
		update_screen(Constants::Constants::get().get_viewwidth(), Constants::Constants::get().get_viewheight());

		build_header();
		build_pages();
		end_draft();
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

		const Texture& panel = draft ? write_panel : view_panel;
		panel.draw(position + VIEW_PANEL_AT + panel.get_origin());

		if (draft)
		{
			for (int32_t slot = 0; slot < GuildBbs::BASIC_ICONS + static_cast<int32_t>(owned_icons.size()); slot++)
			{
				Texture icon = icon_texture(GuildBbs::slot_icon(slot, owned_icons));
				Point<int16_t> at = PALETTE_ICON_AT + Point<int16_t>(GuildBbs::PALETTE_PITCH * slot, 0);

				icon.draw(position + at + icon.get_origin());
			}

			Point<int16_t> frame = PALETTE_FRAME_AT + Point<int16_t>(GuildBbs::PALETTE_PITCH * highlight, 0);
			select_frame.draw(position + frame + select_frame.get_origin());
		}

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

		if (thread && !draft)
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

		// The replies' scrollbar stays with the write panel.
		reply_slider.draw(position);

		if (draft)
		{
			title_field.draw(position);
			body_field.draw(position);
			draft_slider.draw(position);
		}
		else
		{
			body_slider.draw(position);
			reply_field.draw(position);
		}

		UIElement::draw(inter);
	}

	void UIGuildBBS::update()
	{
		UIElement::update();

		reply_field.update(position);
		title_field.update(position);
		body_field.update(position);

		if (draft)
		{
			draft_slider.setrows(static_cast<int16_t>(body_field.first_line()), GuildBbs::BODY_ROWS,
					static_cast<int16_t>(body_field.line_count()));
		}
	}

	Texture UIGuildBBS::icon_texture(int32_t id) const
	{
		if (id >= 0 && id < static_cast<int32_t>(basic_icons.size()))
			return basic_icons[id];

		if (id >= GuildBbs::FIRST_CASH_ICON && id < GuildBbs::FIRST_CASH_ICON + static_cast<int32_t>(cash_icons.size()))
			return cash_icons[id - GuildBbs::FIRST_CASH_ICON];

		return Texture();
	}

	UIGuildBBS::Row UIGuildBBS::make_row(const GuildBbs::Entry& entry) const
	{
		Row row;
		row.text = plain_text(Text::Font::A12M, Color::Name::BLACK, GuildBbs::row_text(entry.id, entry.title));
		row.count = plain_text(Text::Font::A12M, Color::Name::PICTONBLUE, GuildBbs::count_text(entry.replies));

		row.icon = icon_texture(entry.icon);
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

		auto enable = [&](uint16_t id, bool enabled)
		{
			buttons[id]->set_state(enabled ? Button::State::NORMAL : Button::State::DISABLED);
		};

		enable(BT_NOTICE, GuildBbs::may_moderate(rank));
		enable(BT_WRITE, true);
		enable(BT_RETOUCH, editable);
		enable(BT_DELETE, editable);
		enable(BT_REPLY, thread.has_value());

		// The reply box is made again, empty.
		reply_field.change_text("");

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
			bool shown = !draft && thread && index < thread->replies.size()
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
		selected = thread->id;
		end_draft();
		build_view();
	}

	void UIGuildBBS::thread_missing()
	{
		UI::get().emplace<UIOk>(THREAD_GONE, [](bool) {});

		thread.reset();
		end_draft();
		build_view();
	}

	void UIGuildBBS::remove_cursor()
	{
		UIElement::remove_cursor();

		body_slider.remove_cursor();
		reply_slider.remove_cursor();
		draft_slider.remove_cursor();
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

		// Every field hears the click, so the others lose the focus.
		Cursor::State field_state = Cursor::State::IDLE;

		for (Textfield* field : { &reply_field, &title_field, &body_field })
		{
			Cursor::State state = field->send_cursor(cursorpos, clicked);

			if (state != Cursor::State::IDLE)
				field_state = state;
		}

		if (field_state != Cursor::State::IDLE)
			return field_state;

		for (Slider* slider : { &body_slider, &reply_slider, &draft_slider })
		{
			if (draft && slider == &body_slider || !draft && slider == &draft_slider)
				continue;

			if (Cursor::State state = slider->send_cursor(local, clicked))
				return state;
		}

		if (pressed && draft)
		{
			int32_t slot = GuildBbs::slot_at(local.x(), local.y(), owned_icons.size());

			if (slot != GuildBbs::NO_ROW)
			{
				highlight = slot;
				icon = GuildBbs::slot_icon(slot, owned_icons);

				return Cursor::State::IDLE;
			}
		}

		if (pressed)
		{
			int32_t row = GuildBbs::row_at(local.x(), local.y(), list.has_notice, list.entries.size());

			if (row != GuildBbs::NO_ROW)
			{
				// The notice is thread 0 (0x81621c).
				int32_t id = row == GuildBbs::NOTICE_ROW ? 0 : list.entries[row].id;

				selected = id;
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
		if (draft && wheel_on_body)
			draft_slider.send_scroll(yoffset);
		else if (draft)
			reply_slider.send_scroll(yoffset);
		else if (wheel_on_body)
			body_slider.send_scroll(yoffset);
		else
			reply_slider.send_scroll(yoffset);
	}

	Button::State UIGuildBBS::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
		case BT_NOTICE:
			start_draft(Draft::NOTICE);
			break;
		case BT_WRITE:
			start_draft(Draft::NEW);
			break;
		case BT_RETOUCH:
			start_draft(Draft::EDIT);
			break;
		case BT_DELETE:
			ask_delete_thread();
			break;
		case BT_REPLY:
			send_reply();
			break;
		case BT_REGISTER:
			submit();
			break;
		case BT_CANCEL:
			cancel_draft();
			break;
		case BT_LEFT:
		case BT_RIGHT:
			highlight = std::clamp(highlight + (buttonid == BT_LEFT ? -1 : 1), 0,
					GuildBbs::BASIC_ICONS + static_cast<int32_t>(owned_icons.size()) - 1);
			break;
		case BT_QUIT:
			close();
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

	void UIGuildBBS::start_draft(Draft kind)
	{
		if (kind == Draft::NOTICE && list.has_notice)
		{
			UI::get().emplace<UIOk>(NOTICE_TAKEN, [](bool) {});

			return;
		}

		if (kind == Draft::EDIT && !thread)
			return;

		const Inventory& inventory = Stage::get().get_player().get_inventory();
		owned_icons.clear();

		for (int32_t i = 0; i < GuildBbs::CASH_ICONS; i++)
		{
			if (inventory.find_item(InventoryType::Id::CASH, GuildBbs::FIRST_ICON_ITEM + i) > 0)
				owned_icons.push_back(GuildBbs::FIRST_CASH_ICON + i);
		}

		bool editing = kind == Draft::EDIT;
		int32_t slot = editing ? GuildBbs::slot_of(thread->icon, owned_icons) : GuildBbs::NO_ROW;

		// An edit keeps the thread's icon if the palette has it.
		highlight = slot != GuildBbs::NO_ROW ? slot : 0;
		icon = slot != GuildBbs::NO_ROW ? thread->icon : 0;
		draft = kind;

		for (uint16_t id : { BT_NOTICE, BT_WRITE, BT_RETOUCH, BT_DELETE, BT_REPLY })
			buttons[id]->set_active(false);

		for (uint16_t k = 0; k < GuildBbs::REPLY_ROWS; k++)
			buttons[BT_REPLY_DELETE0 + k]->set_active(false);

		for (uint16_t id : { BT_REGISTER, BT_CANCEL, BT_LEFT, BT_RIGHT })
			buttons[id]->set_active(true);

		reply_field.set_state(Textfield::State::DISABLED);

		// Nothing takes the focus until a click (0x8145a9).
		title_field.change_text(editing ? thread->title : "");
		body_field.change_text(editing ? thread->body : "");
		title_field.set_state(Textfield::State::NORMAL);
		body_field.set_state(Textfield::State::NORMAL);
		body_field.scroll_lines(0);
	}

	void UIGuildBBS::end_draft()
	{
		draft.reset();

		title_field.set_state(Textfield::State::DISABLED);
		body_field.set_state(Textfield::State::DISABLED);

		for (uint16_t id : { BT_REGISTER, BT_CANCEL, BT_LEFT, BT_RIGHT })
			buttons[id]->set_active(false);

		for (uint16_t id : { BT_NOTICE, BT_WRITE, BT_RETOUCH, BT_DELETE, BT_REPLY })
			buttons[id]->set_active(true);

		// A focused reply box keeps its focus.
		if (reply_field.get_state() == Textfield::State::DISABLED)
			reply_field.set_state(Textfield::State::NORMAL);
	}

	void UIGuildBBS::submit()
	{
		if (!draft || *draft == Draft::EDIT && !thread)
			return;

		const std::string& subject = title_field.get_text();
		const std::string& text = body_field.get_text();

		if (const char* refusal = refusal_text(GuildBbs::check_thread(subject, text, CurseData::filter)))
		{
			UI::get().emplace<UIOk>(refusal, [](bool) {});

			return;
		}

		std::optional<int32_t> edited = *draft == Draft::EDIT ? std::optional<int32_t>(thread->id) : std::nullopt;
		BbsWritePacket(edited, *draft == Draft::NOTICE, subject, text, icon).dispatch();

		// The panel stays until the thread comes back (mode 7); the list
		// goes back to its first page.
		page = 0;
		BbsListPacket(page).dispatch();
	}

	void UIGuildBBS::cancel_draft()
	{
		end_draft();
		build_view();

		// 0x8168ca
		BbsShowPacket(selected).dispatch();
		BbsListPacket(page).dispatch();
	}

	void UIGuildBBS::send_reply()
	{
		if (!thread || draft)
			return;

		const std::string& text = reply_field.get_text();

		if (const char* refusal = refusal_text(GuildBbs::check_reply(text, CurseData::filter)))
		{
			UI::get().emplace<UIOk>(refusal, [](bool) {});

			return;
		}

		// The thread comes back with the reply (0x816994).
		BbsReplyPacket(thread->id, text).dispatch();
		BbsListPacket(page).dispatch();
	}

	void UIGuildBBS::close()
	{
		for (Textfield* field : { &reply_field, &title_field, &body_field })
			field->set_state(Textfield::State::DISABLED);

		deactivate();
	}

	UIElement::Type UIGuildBBS::get_type() const
	{
		return TYPE;
	}
}
