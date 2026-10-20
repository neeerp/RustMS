#pragma once

#include "../UIElement.h"

#include "../Components/Slider.h"

#include "../../Graphics/Text.h"
#include "../../Graphics/Texture.h"
#include "../../Util/Memos.h"

#include <vector>

namespace ms
{
	// v83's inbox (UIWindow.img/Memo/backgrnd, FUN_00649d82, report 204m):
	// every note waiting, each with who sent it and when, which the window
	// takes for its own. Closing it, after a warning, tells the server they
	// were all read; there is no keeping some.
	class UIMemo : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::MEMO;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		explicit UIMemo(std::vector<Memos::Memo> memos);

		// Open the inbox with the notes waiting, if any (FUN_0052448a).
		static void open();

		void draw(float inter) const override;
		void update() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;
		void send_key(int32_t keycode, bool pressed, bool escape) override;
		void send_scroll(double yoffset) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		enum Buttons : uint16_t { OK, CLOSE };

		struct Glyph
		{
			Texture texture;
			int16_t x;
		};

		struct Row
		{
			Text sender;
			std::vector<Glyph> date;
			std::vector<Text> lines;
			// A thank-you's or a gift's lines about the gift and fame.
			std::vector<Text> extra;
			// Where it starts in the list, and its height.
			int16_t y;
			int16_t height;
			uint8_t flag;
		};

		void ask_to_close();
		void finish();
		void scroll_to(int16_t step);

		// Draw `texture` at `at`, as much of it as lies in the list's view.
		void draw_clipped(const Texture& texture, Point<int16_t> at) const;
		Range<int16_t> view() const;

		std::vector<Memos::Memo> memos;
		std::vector<Row> rows;

		Texture background, inner_top, inner_center, inner_bottom, line;
		Text heading, count, tail;
		Slider slider;
		int16_t step = 0;
		bool warned = false;
	};
}
