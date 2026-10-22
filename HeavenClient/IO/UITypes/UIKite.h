#pragma once

#include "../UIElement.h"

#include "../Components/Textfield.h"

#include "../../Gameplay/MapleMap/KiteRules.h"
#include "../../Graphics/Text.h"

#include <array>
#include <functional>
#include <string>
#include <vector>

namespace ms
{
	// A kite's entry window (v83 FUN_007dbb5e on UIWindow.img/MultiLine/backgrnd2): the item's `msgIn` and three
	// lines of up to 60 characters. It is modal; Enter does nothing, and Escape cancels.
	class UIKiteWrite : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::KITEWRITE;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		// For kite `item_id`. `onsend` gets the three lines as typed.
		UIKiteWrite(int32_t item_id, std::function<void(const std::vector<std::string> &lines)> onsend);

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

		std::function<void(const std::vector<std::string> &lines)> onsend;
		Text prompt;
		// Fields don't move once made: the UI keeps a pointer to the one with the keys.
		std::array<Textfield, kite::LINES> fields;
	};

	// Someone's kite, opened by double-clicking it (v83 FUN_00658e59 on UIWindow.img/MultiLine/backgrnd): the
	// flier's name, the item's `msg` and the lines. Escape or Enter closes it.
	class UIKiteView : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::KITEVIEW;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		UIKiteView(int32_t item_id, const std::string &name, const std::string &message);

		void draw(float alpha) const override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		enum Buttons : uint16_t
		{
			CLOSE
		};

		Text name;
		Text item_msg;
		std::vector<Text> lines;
	};
}
