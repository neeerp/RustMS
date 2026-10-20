#pragma once

#include "../UIElement.h"

#include "../Components/ComboEdit.h"

#include "../../Graphics/Text.h"
#include "../../Graphics/Texture.h"

#include <functional>
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

		enum Buttons : uint16_t
		{
			OK,
			CANCEL
		};

		std::function<void(const std::string &)> okhandler;
		Texture top;
		Texture middle;
		Texture bottom;
		Text prompt;
		ComboEdit combo;
	};
}
