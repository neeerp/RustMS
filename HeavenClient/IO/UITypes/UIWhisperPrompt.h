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
	// v83's prompt for whom to whisper (FUN_008d4dbf): a compact
	// CUtilDlgEx whose combo box offers the names last whispered to. A
	// right click drops a name from the list, kept only if OK is pressed.
	// OK hands over the text and the list, and always closes.
	class UIWhisperPrompt : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::NOTICE;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		using OkHandler = std::function<void(const std::string &name, const std::vector<std::string> &history)>;

		UIWhisperPrompt(std::vector<std::string> history, OkHandler okhandler);

		void draw(float alpha) const override;

		void update() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void rightclick(Point<int16_t> cursorpos) override;

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

		OkHandler okhandler;
		Texture top;
		Texture middle;
		Texture bottom;
		Text prompt;
		ComboEdit combo;
		bool dragged = false;
		Point<int16_t> cursoroffset;
	};
}
