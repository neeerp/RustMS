#pragma once

#include "../UIElement.h"

#include "../Components/Textfield.h"

#include <functional>
#include <string>

namespace ms
{
	// The Maplemas Party Potion's dialog (v83 FUN_00a52f31 on UIWindow.img/Transform): whom to transform, a
	// name of up to 12 characters. OK or Enter sends a name typed, and closes the dialog; Cancel or Escape
	// closes it (FUN_00a532ac, FUN_00a532f2).
	class UITransform : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::TRANSFORM;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		// `onsend` gets the name; it returns false to keep the dialog open.
		UITransform(std::function<bool(const std::string &name)> onsend);

		void draw(float alpha) const override;

		void update() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		void send();

		void close();

		enum Buttons : uint16_t
		{
			OK,
			CANCEL
		};

		std::function<bool(const std::string &name)> onsend;
		Textfield field;
	};
}
