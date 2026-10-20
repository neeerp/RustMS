#pragma once

#include "../UIElement.h"

#include "../Components/Textfield.h"

#include "../../Graphics/Text.h"
#include "../../Graphics/Texture.h"

#include <functional>
#include <optional>
#include <string>
#include <vector>

namespace ms
{
	// v83's "To Group" dialog (FUN_008d500b): a CUtilDlg asking for a buddy
	// group in a combo box whose text is typed or picked from the groups.
	// OK hands over the text, blank or not.
	class UIGroupPicker : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::NOTICE;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		UIGroupPicker(std::vector<std::string> choices, std::function<void(const std::string &group)> okhandler);

		void draw(float alpha) const override;

		void update() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		void ok();
		void close();
		void open_list();
		void pick(size_t row);
		void move_hover(bool down);
		// Takes the list's keys while it is open.
		bool list_key(int32_t keycode);
		std::optional<size_t> row_at(Point<int16_t> pos) const;
		Rectangle<int16_t> list_bounds() const;

		enum Buttons : uint16_t
		{
			OK,
			CANCEL
		};

		enum Face : uint8_t
		{
			NORMAL,
			MOUSEOVER,
			PRESSED,
			NUM_FACES
		};

		std::function<void(const std::string &)> okhandler;
		Texture top;
		Texture middle;
		Texture bottom;
		Texture faces[NUM_FACES][3];
		Text prompt;
		Textfield edit;
		std::vector<std::string> choices;
		std::vector<Text> rows;
		std::vector<Text> lit_rows;
		bool list_open = false;
		size_t selected = 0;
		size_t hover = 0;
		bool combo_hovered = false;
		uint16_t pressed_time = 0;
		bool mouse_down = false;
	};
}
