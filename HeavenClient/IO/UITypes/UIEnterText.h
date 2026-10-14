#pragma once

#include "UINotice.h"

#include <functional>

namespace ms
{
	// A notice with a text field, e.g. for the name of a character to invite.
	class UIEnterText : public UINotice
	{
	public:
		UIEnterText(std::string message, std::function<void(const std::string& text)> texthandler, size_t limit);

		void draw(float alpha) const override;

		void update() override;

		Cursor::State send_cursor(bool pressed, Point<int16_t> cursorpos) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		void submit();

		enum Buttons : int16_t
		{
			OK, CANCEL
		};

		std::function<void(const std::string& text)> texthandler;
		Textfield textfield;
	};
}
