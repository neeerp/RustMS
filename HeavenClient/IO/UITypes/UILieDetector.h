#pragma once

#include "../UIElement.h"

#include "../Components/Textfield.h"

#include "../../Graphics/PixelTexture.h"
#include "../../Graphics/Text.h"

#include <array>
#include <functional>
#include <string>
#include <vector>

namespace ms
{
	// The Lie Detector's name prompt (v83 FUN_0096d233 on UIWindow.img/Macro/backgrnd): whom to test. OK or
	// Enter passes on a name typed and closes it; Cancel or Escape closes it.
	class UILieDetectorName : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::LIEDETECTORNAME;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		UILieDetectorName(std::function<void(const std::string &name)> onsend);

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

		std::function<void(const std::string &name)> onsend;
		Textfield field;
	};

	// The tested player's challenge (v83 dialog A, 0x7e65de, on UIWindow.img/Macro/backgrnd4): the picture,
	// the time left and the tries left. OK or Enter sends what was typed and closes it; it can't be cancelled,
	// and when the time runs out the server ends the test (0x7e7603).
	class UILieDetector : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::LIEDETECTOR;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		// `first` says whether this is the test's first picture.
		UILieDetector(PixelTexture picture, int32_t time_left, bool first);

		void draw(float alpha) const override;

		void update() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		void send();

		enum Buttons : uint16_t
		{
			OK
		};

		PixelTexture picture;
		int64_t deadline;
		Textfield field;
		std::array<Texture, 10> digits;
		Texture comma;
		Text tries;
	};

	// What came of a Lie Detector (v83 FUN_007e8a2f): a GM's face and a few lines, closed with OK.
	class UILieDetectorResult : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::LIEDETECTORRESULT;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		UILieDetectorResult(int portrait, const std::vector<std::string> &lines);

		void draw(float alpha) const override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		enum Buttons : uint16_t
		{
			OK
		};

		Text text;
	};
}
