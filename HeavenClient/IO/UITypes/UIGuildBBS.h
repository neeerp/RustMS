#pragma once

#include "../UIElement.h"

#include "../Components/Slider.h"
#include "../Components/Textfield.h"

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
	// clicked and its replies, or the panel writing a thread. The window only
	// asks: every change waits for the server's list (mode 6), thread (7),
	// or "thread gone" (8).
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

		void update() override;

		void update_screen(int16_t new_width, int16_t new_height) override;

		void remove_cursor() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_scroll(double yoffset) override;

		UIElement::Type get_type() const override;

		// Mode 6: the list is replaced; the thread view stays.
		void show_list(const GuildBbs::List& list);
		// Mode 7: the thread view is rebuilt, ending any writing.
		void show_thread(GuildBbs::Thread thread);
		// Mode 8: the thread is dropped, ending any writing.
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
			BT_QUIT,
			BT_REGISTER,
			BT_CANCEL,
			BT_LEFT,
			BT_RIGHT
		};

		// What the write panel writes (0x8165e2).
		enum class Draft
		{
			NEW,
			EDIT,
			NOTICE
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

		Texture icon_texture(int32_t icon) const;
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
		// Opens the write panel (0x8165e2), unless a second notice.
		void start_draft(Draft kind);
		// Back to the list and view's controls (0x813a8c).
		void end_draft();
		void submit();
		void cancel_draft();
		void send_reply();
		void close();

		Texture background;
		Texture view_panel;
		Texture write_panel;
		Texture select_frame;
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

		// The thread asked for last; Cancel asks for it again.
		int32_t selected;
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
		Textfield reply_field;

		std::optional<Draft> draft;
		Textfield title_field;
		Textfield body_field;
		Slider draft_slider;
		// The cash icons owned, which the palette offers after the free ones.
		std::vector<int32_t> owned_icons;
		// The palette's highlighted slot, and the icon sent: the arrows move
		// only the first (0x81649b).
		int32_t highlight;
		int32_t icon;

		// Clicks arrive with every cursor move while the button is down.
		bool mouse_down;
		// Where the cursor last was, as the wheel arrives without it.
		bool wheel_on_body;
	};
}
