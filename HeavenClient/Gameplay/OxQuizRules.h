#pragma once

#include <cstdint>
#include <string>

namespace ms
{
	// OX_QUIZ (FUN_00535a57): a question of Etc.nx OXQuiz.img, or its answer, on the status bar's second float notice.
	namespace ox_quiz
	{
		enum class Show
		{
			CLEAR,
			KEEP,
			QUESTION,
			ANSWER
		};

		// Question 0, no OXQuiz.img or no such question take the notice down; a set that is not there leaves it
		// as it is. A flag shows the question, none its answer.
		inline Show show(uint8_t flag, uint16_t question, bool img_found, bool set_found, bool question_found)
		{
			if (question == 0 || !img_found)
				return Show::CLEAR;

			if (!set_found)
				return Show::KEEP;

			if (!question_found)
				return Show::CLEAR;

			return flag != 0 ? Show::QUESTION : Show::ANSWER;
		}

		// The answer's line: its explanation, a space if there is one, and "[Answer:O]" or "[Answer:X]"
		// (StringPool 0x38d, 0x8ac and 0x8ad).
		inline std::string answer_line(const std::string &explanation, int32_t answer)
		{
			std::string line = explanation;

			if (!line.empty())
				line += " ";

			return line + "[Answer:" + (answer != 0 ? "O" : "X") + "]";
		}

		// Each question puts up an event clock of this many seconds if none is up, and starts it.
		constexpr int32_t CLOCK_SECONDS = 30;

		// The float type of the notice's art.
		constexpr int32_t FLOAT_TYPE = 0;
	}

	// GMEVENT_INSTRUCTIONS (FUN_005360c0): one of the map's String.nx help lines, read at CField::Init as help0,
	// help1, ... up to the first missing one (FUN_0052a453).
	namespace gm_event
	{
		// The line at the index, or none when it, or one before it, is missing. The lookup gives a help line by
		// number, empty for none.
		template<class Lookup>
		std::string help_line(uint8_t index, Lookup line)
		{
			for (uint8_t i = 0; i < index; i++)
				if (line(i).empty())
					return {};

			return line(index);
		}

		// The Maple Administrator speaks them.
		constexpr int32_t NPC = 9010000;
	}
}
