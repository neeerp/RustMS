#pragma once

#include "../UIElement.h"

#include "../Components/ComboEdit.h"
#include "../Components/Textfield.h"

#include "../../Graphics/Text.h"
#include "../../Graphics/Texture.h"

#include <functional>
#include <string>
#include <vector>

namespace ms
{
	// v83's "To Group" dialog (FUN_008d500b): a CUtilDlg asking for a buddy
	// group in a combo box whose text is typed or picked from the groups.
	// OK hands over the text, blank or not. The Add Buddy dialog (kind 0x3ea,
	// FUN_00909fca) is the same with a name edit under the combo.
	class UIGroupPicker : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::NOTICE;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		UIGroupPicker(std::vector<std::string> choices, std::function<void(const std::string &group)> okhandler);
		// Add Buddy, its name starting as name; OK hands over the name
		// trimmed and the group as typed.
		UIGroupPicker(const std::string &name, std::vector<std::string> choices,
					  std::function<void(const std::string &name, const std::string &group)> okhandler);

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

		UIGroupPicker(bool asks_name, std::vector<std::string> choices);

		bool asks_name;
		std::function<void(const std::string &)> okhandler;
		std::function<void(const std::string &, const std::string &)> name_okhandler;
		Texture top;
		Texture middle;
		Texture bottom;
		std::vector<Text> prompt;
		ComboEdit combo;
		Textfield name;
	};
}
