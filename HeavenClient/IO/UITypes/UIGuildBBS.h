#pragma once

#include "../UIElement.h"

#include "../Components/Slider.h"

#include "../../Character/GuildBbs.h"
#include "../../Character/GuildMark.h"
#include "../../Graphics/Text.h"
#include "../../Graphics/Texture.h"

#include <array>
#include <cstdint>
#include <optional>
#include <vector>

namespace ms
{
	// The guild bulletin board (v83 CUIGuildBBS, ctor 0x812fb4; spec
	// v83ref/scratch/251/bbs.md), which the Guild tab's BtGuildBBS opens. The
	// left shows the notice and a page of threads, the right the thread
	// clicked and its replies. The window only asks: every change waits for
	// the server's list (mode 6), thread (7), or "thread gone" (8).
	// Writing, editing, the notice, and replying are not built yet; their
	// buttons stay disabled.
	class UIGuildBBS : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::GUILDBBS;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		// Opens the board unless it is open; it asks for page 0.
		static void open();

		UIGuildBBS();

		void draw(float inter) const override;

		void update_screen(int16_t new_width, int16_t new_height) override;

		void remove_cursor() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_scroll(double yoffset) override;

		UIElement::Type get_type() const override;

		// Mode 6: the list is replaced; the thread view stays.
		void show_list(const GuildBbs::List& list);
		// Mode 7: the thread view is rebuilt.
		void show_thread(GuildBbs::Thread thread);
		// Mode 8: the thread is dropped.
		void thread_missing();

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		enum Buttons : uint16_t
		{
			BT_NOTICE,
			BT_WRITE,
			BT_RETOUCH,
			BT_DELETE,
			BT_REPLY,
			BT_REPLY_DELETE0,
			BT_REPLY_DELETE3 = BT_REPLY_DELETE0 + 3,
			BT_QUIT
		};

		struct Row
		{
			Text text;
			Text count;
			Texture icon;
			Text writer;
			Text date;
		};

		struct ReplyRow
		{
			Text writer;
			Text text;
			Text date;
		};

		struct PageLink
		{
			int32_t page;
			Text label;
			int16_t left;
			int16_t text_x;
			// The click area: a number's whole cell.
			int16_t width;
		};

		Row make_row(const GuildBbs::Entry& entry) const;
		void draw_row(const Row& row, int16_t y) const;
		void build_header();
		void build_pages();
		// Rebuilds the view and its buttons, as 0x813a8c does.
		void build_view();
		void show_reply_buttons();
		void scroll_body(int16_t to);
		void scroll_replies(int16_t to);
		void ask_delete_thread();
		void ask_delete_reply(size_t row);

		Texture background;
		Texture view_panel;
		Texture guild_frame;
		std::array<Texture, 3> basic_icons;
		std::array<Texture, 7> cash_icons;

		GuildMark emblem;
		bool emblem_shown;
		Text guild_name;

		GuildBbs::List list;
		std::optional<Row> notice_row;
		std::vector<Row> rows;
		int32_t page;
		std::vector<PageLink> page_links;

		std::optional<GuildBbs::Thread> thread;
		Text title;
		Text poster;
		Text posted;
		std::vector<Text> body;
		int16_t body_scroll;
		Slider body_slider;
		std::vector<ReplyRow> replies;
		int16_t reply_scroll;
		Slider reply_slider;

		// Clicks arrive with every cursor move while the button is down.
		bool mouse_down;
		// Where the cursor last was, as the wheel arrives without it.
		bool wheel_on_body;
	};
}
