#pragma once

#include "../UIElement.h"

#include "../Components/Textfield.h"

#include "../../Util/MapleTv.h"

#include <array>
#include <functional>
#include <string>

namespace ms
{
	// Maple TV's dialog (v83 FUN_007f00f1 on UIWindow.img/MapleTV, report
	// c056 section 3): who the message is for, if its kind names anyone, and
	// one box whose first five wrapped rows are the message. A Messenger
	// names someone only with its checkbox ticked, and a Megassenger has the
	// ear. It is modal, and closes before the message is checked.
	class UIMapleTv : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::MAPLETV;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		using OnSend = std::function<void(const std::string &receiver, const MapleTv::Lines &lines, bool ear)>;

		UIMapleTv(int32_t item_id, OnSend onsend);

		void draw(float alpha) const override;

		void update() override;

		void remove_cursor() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		void send();

		void close();

		// The Messenger's checkbox: whether the receiver may be named.
		void set_addressed(bool addressed);

		enum Buttons : uint16_t
		{
			OK,
			CANCEL
		};

		// What the checkboxes are.
		enum Check : uint8_t
		{
			ADDRESSED,
			EAR,
			NONE
		};

		MapleTv::Mode mode;
		bool mega;
		OnSend onsend;
		Textfield text;
		Textfield receiver;
		std::array<Texture, 2> checkbox;
		std::array<Texture, 2> to_bar;
		bool addressed;
		bool ear;
		bool held;
		// The checkbox the button went down on, which toggles when it comes up.
		Check checking;
	};
}
